use super::support::*;
use domain::job::JobStatus;
use jobs::queue::JobQueueConfig;
use ports::{
    job_resources::ResourceDemand, job_runtime_control::JobRuntimeControlPort,
    job_scheduler::JobSchedulerPort,
};
use std::time::Duration;

fn config() -> JobQueueConfig {
    JobQueueConfig {
        budgets: ResourceDemand {
            cpu: 1,
            network: 1,
            disk: 1,
            processes: 1,
        },
        ..Default::default()
    }
}

#[tokio::test]
async fn cancelling_pending_excludes_dispatch_and_repeated_cancel_is_idempotent() {
    let harness = Harness::new(config()).await;
    let mut events = harness.controlled().await;
    harness.admit(CPU).await;
    let running = started(&mut events).await;
    let waiting = harness.admit(CPU).await;
    let cancelled = harness.manager.cancel_job(&waiting).await.unwrap();
    let again = harness.manager.cancel_job(&waiting).await.unwrap();
    assert_eq!(cancelled.revision, again.revision);
    assert_eq!(again.status, JobStatus::Cancelled);
    let survivor = harness.admit(CPU).await;
    running.finish.send(true).unwrap();
    let next = started(&mut events).await;
    assert_eq!(next.id, survivor);
    next.finish.send(true).unwrap();
    harness.wait_status(&survivor, JobStatus::Completed).await;
    harness.shutdown().await;
}

#[tokio::test]
async fn running_cancel_waits_for_cleanup_and_capacity_cannot_be_reissued() {
    let harness = Harness::new(config()).await;
    let mut events = harness.controlled().await;
    let first = harness.admit(CPU).await;
    let running = started(&mut events).await;
    let waiting = harness.admit(CPU).await;
    let manager = harness.manager.clone();
    let id = first.clone();
    let cancel = tokio::spawn(async move { manager.cancel_job(&id).await });
    harness.wait_status(&first, JobStatus::Cancelling).await;
    assert!(!cancel.is_finished());
    assert_eq!(harness.status(&waiting).await, JobStatus::Pending);
    assert_eq!(harness.manager.queue_available_resources().unwrap().cpu, 0);
    running.cleanup.send(()).unwrap();
    assert_eq!(cancel.await.unwrap().unwrap().status, JobStatus::Cancelled);
    let next = started(&mut events).await;
    assert_eq!(next.id, waiting);
    next.finish.send(true).unwrap();
    harness.wait_status(&waiting, JobStatus::Completed).await;
    harness.shutdown().await;
}

#[tokio::test]
async fn timed_out_cancel_retains_capacity_until_confirmed_cleanup_and_retry_clears_attention() {
    use ports::repository::JobRepository;
    let harness = Harness::new(JobQueueConfig {
        cleanup_timeout: Duration::from_millis(100),
        ..config()
    })
    .await;
    let mut events = harness.controlled().await;
    let first = harness.admit(CPU).await;
    let running = started(&mut events).await;
    let pending = harness.admit(CPU).await;
    assert!(matches!(
        harness.manager.cancel_job(&first).await,
        Err(ports::PortError::CleanupFailed { .. })
    ));
    let interrupted = harness.jobs.get(&first).await.unwrap().unwrap();
    assert_eq!(interrupted.status(), &JobStatus::Cancelling);
    assert_eq!(interrupted.error().unwrap().code, "CLEANUP_UNCONFIRMED");
    assert_eq!(harness.status(&pending).await, JobStatus::Pending);
    assert_eq!(harness.manager.queue_available_resources().unwrap().cpu, 0);
    running.cleanup.send(()).unwrap();
    harness.manager.cancel_job(&first).await.unwrap();
    let cancelled = harness.jobs.get(&first).await.unwrap().unwrap();
    assert_eq!(cancelled.status(), &JobStatus::Cancelled);
    assert!(cancelled.error().is_none());
    assert!(cancelled.progress().current_step.is_none());
    let next = started(&mut events).await;
    assert_eq!(next.id, pending);
    next.finish.send(true).unwrap();
    harness.wait_status(&pending, JobStatus::Completed).await;
    harness.shutdown().await;
}

#[tokio::test]
async fn cancel_dispatch_race_never_executes_twice() {
    let harness = Harness::new(config()).await;
    let mut events = harness.controlled().await;
    for _ in 0..32 {
        harness.admit(CPU).await;
        let running = started(&mut events).await;
        let waiting = harness.admit(CPU).await;
        let manager = harness.manager.clone();
        let id = waiting.clone();
        let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));
        let release_barrier = barrier.clone();
        let release = tokio::spawn(async move {
            release_barrier.wait().await;
            running.finish.send(true).unwrap();
        });
        let mut cancel = tokio::spawn(async move {
            barrier.wait().await;
            manager.cancel_job(&id).await
        });
        release.await.unwrap();
        let next = harness.admit(NETWORK).await;
        let mut next_seen = false;
        let cancelled = loop {
            tokio::select! {
                result = &mut cancel => break result.unwrap().unwrap(),
                event = events.recv() => {
                    let event = event.unwrap();
                    if event.id == waiting { event.cleanup.send(()).unwrap(); }
                    else { assert_eq!(event.id, next); next_seen = true; event.finish.send(true).unwrap(); }
                }
            }
        };
        if !next_seen {
            let event = started(&mut events).await;
            assert_eq!(event.id, next);
            event.finish.send(true).unwrap();
        }
        assert_eq!(cancelled.status, JobStatus::Cancelled);
        harness.wait_status(&next, JobStatus::Completed).await;
    }
    harness.shutdown().await;
}

#[tokio::test]
async fn shutdown_deadline_preserves_unconfirmed_worker_and_pending_descriptor() {
    let harness = Harness::new(config()).await;
    let mut events = harness.controlled().await;
    let first = harness.admit(CPU).await;
    let running = started(&mut events).await;
    let pending = harness.admit(CPU).await;
    let mut lifecycle = harness.events.subscribe();
    let before = std::time::Instant::now();
    tokio::time::pause();
    let drain = harness.manager.drain_all(Duration::from_millis(100));
    tokio::pin!(drain);
    loop {
        assert!(futures::poll!(drain.as_mut()).is_pending());
        if lifecycle
            .try_recv()
            .is_ok_and(|event| event.job.id == first && event.job.status == JobStatus::Cancelling)
        {
            break;
        }
        assert!(before.elapsed() < Duration::from_secs(1));
        tokio::task::yield_now().await;
    }
    tokio::time::advance(Duration::from_millis(100)).await;
    let report = drain.await.unwrap();
    tokio::time::resume();
    assert_eq!(report.unconfirmed_count, 1);
    assert!(before.elapsed() < Duration::from_secs(1));
    assert_eq!(harness.status(&first).await, JobStatus::Cancelling);
    assert_eq!(harness.status(&pending).await, JobStatus::Pending);
    assert_eq!(harness.manager.queue_available_resources().unwrap().cpu, 0);
    let cancel_manager = harness.manager.clone();
    let cancel_id = first.clone();
    let cancel = tokio::spawn(async move { cancel_manager.cancel_job(&cancel_id).await });
    running.cleanup.send(()).unwrap();
    cancel.await.unwrap().unwrap();
    assert_eq!(harness.manager.queue_available_resources().unwrap().cpu, 1);
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn shutdown_deadline_with_a_blocked_writer_retains_unconfirmed_ownership() {
    let harness = Harness::new(config()).await;
    let mut events = harness.controlled().await;
    let first = harness.admit(CPU).await;
    let running = started(&mut events).await;
    let pending = harness.admit(CPU).await;
    let writer = harness.pool.begin_with("BEGIN IMMEDIATE").await.unwrap();
    let before = std::time::Instant::now();
    let report = harness
        .manager
        .drain_all(Duration::from_millis(100))
        .await
        .unwrap();
    assert_eq!(report.unconfirmed_count, 1);
    assert!(before.elapsed() < Duration::from_secs(1));
    assert_eq!(harness.status(&first).await, JobStatus::Running);
    assert_eq!(harness.status(&pending).await, JobStatus::Pending);
    assert_eq!(harness.manager.queue_available_resources().unwrap().cpu, 0);
    writer.rollback().await.unwrap();
    let cancel_manager = harness.manager.clone();
    let cancel_id = first.clone();
    let cancel = tokio::spawn(async move { cancel_manager.cancel_job(&cancel_id).await });
    running.cleanup.send(()).unwrap();
    cancel.await.unwrap().unwrap();
    assert_eq!(harness.manager.queue_available_resources().unwrap().cpu, 1);
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn project_deletion_removes_pending_and_joins_running_cleanup() {
    use application::usecases::project::{
        delete::{DeleteProjectRequest, DeleteProjectUseCase},
        lifecycle::ProjectLifecycleLocks,
    };
    use ports::repository::JobRepository;
    let harness = Harness::new(config()).await;
    let mut events = harness.controlled().await;
    let first = harness.admit(CPU).await;
    let running = started(&mut events).await;
    let pending = harness.admit(CPU).await;
    let pending_project = harness
        .jobs
        .get(&pending)
        .await
        .unwrap()
        .unwrap()
        .project_id()
        .clone();
    let delete = DeleteProjectUseCase::new(
        harness.uow.clone(),
        harness.manager.clone(),
        std::sync::Arc::new(ProjectLifecycleLocks::new()),
    );
    delete
        .execute(DeleteProjectRequest {
            project_id: pending_project,
        })
        .await
        .unwrap();
    assert!(harness.jobs.get(&pending).await.unwrap().is_none());
    let running_project = harness
        .jobs
        .get(&first)
        .await
        .unwrap()
        .unwrap()
        .project_id()
        .clone();
    let deletion = tokio::spawn(async move {
        delete
            .execute(DeleteProjectRequest {
                project_id: running_project,
            })
            .await
    });
    // The fixture observes cancellation via its cleanup acknowledgement; deletion cannot finish first.
    running.cleanup.send(()).unwrap();
    deletion.await.unwrap().unwrap();
    assert!(harness.jobs.get(&first).await.unwrap().is_none());
    assert_eq!(harness.manager.queue_available_resources().unwrap().cpu, 1);
    harness.shutdown().await;
}
