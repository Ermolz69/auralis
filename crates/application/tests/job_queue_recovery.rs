#![allow(clippy::unwrap_used)]

#[path = "support/queue_subtitles.rs"]
mod subtitles;
#[path = "support/job_queue.rs"]
mod support;

use application::usecases::system::recover_interrupted::usecase::RecoverInterruptedStateUseCase;
use domain::job::{Job, JobStatus};
use jobs::queue::JobQueueConfig;
use ports::{
    job_execution::{ExecutionDescriptor, JobExecutionStore},
    repository::JobRepository,
    transaction::CommitPipelineStart,
};
use support::Harness;

async fn persist(harness: &Harness) -> Job {
    let mut project = harness.project().await;
    let job = Job::new(
        project.id().clone(),
        "Recovery fixture".into(),
        domain::job::JobKind::Dubbing,
    );
    project.start_processing(job.id().clone()).unwrap();
    let descriptor =
        ExecutionDescriptor::subtitle_import(project.id().clone(), Some(subtitles::track()));
    harness
        .store
        .admit_pipeline(
            CommitPipelineStart {
                project,
                job: job.clone(),
            },
            descriptor,
            16,
        )
        .await
        .unwrap();
    job
}

#[tokio::test]
async fn reopening_sqlite_preserves_pending_inputs_and_dispatches_only_after_reconciliation() {
    let mut harness = Harness::new(JobQueueConfig::default()).await;
    let job = persist(&harness).await;
    let original = harness.store.descriptor(job.id()).await.unwrap().unwrap();
    harness.pool.close().await;
    let pool = adapters_storage::sqlite::connect_sqlite(harness.root.path().join("auralis.sqlite"))
        .await
        .unwrap();
    harness.pool = pool.clone();
    harness.projects = std::sync::Arc::new(adapters_storage::sqlite::SqliteProjectRepository::new(
        pool.clone(),
    ));
    harness.jobs = std::sync::Arc::new(adapters_storage::sqlite::SqliteJobRepository::new(
        pool.clone(),
    ));
    harness.uow = std::sync::Arc::new(adapters_storage::sqlite::SqliteStorageUnitOfWork::new(
        pool.clone(),
    ));
    harness.store = std::sync::Arc::new(
        adapters_storage::sqlite::job_executions::SqliteJobExecutionStore::new(pool.clone()),
    );
    harness.manager = std::sync::Arc::new(
        jobs::manager::JobManager::new(harness.jobs.clone(), harness.uow.clone(), None)
            .with_queue(harness.store.clone(), JobQueueConfig::default())
            .unwrap(),
    );
    let recovery = RecoverInterruptedStateUseCase::new(std::sync::Arc::new(
        adapters_storage::sqlite::recovery::SqliteRecoveryStorage::new(pool),
    ))
    .with_execution_store(harness.store.clone());
    let report = recovery.execute().await.unwrap();
    assert!(!report.has_blocking_failures());
    assert_eq!(report.actions_applied, 0);
    assert_eq!(harness.status(job.id()).await, JobStatus::Pending);
    assert_eq!(
        original,
        harness.store.descriptor(job.id()).await.unwrap().unwrap()
    );
    let (send, mut downloads) = tokio::sync::mpsc::unbounded_channel();
    let pipeline = harness.pipeline(subtitles::FixtureSubtitles(send), harness.workspace());
    assert!(downloads.try_recv().is_err());
    harness
        .manager
        .start_dispatcher(pipeline.queued_executor())
        .await
        .unwrap();
    let download = subtitles::downloaded(&mut downloads).await;
    assert_eq!(download.track, subtitles::track());
    download.release.send(true).unwrap();
    // Reopened composition has no emitter; observe the durable row with bounded yields.
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while harness.status(job.id()).await != JobStatus::Completed {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    harness.shutdown().await;
}

#[tokio::test]
async fn invalid_or_missing_descriptor_is_explicit_and_old_running_is_never_repeated() {
    let harness = Harness::new(JobQueueConfig::default()).await;
    let invalid = persist(&harness).await;
    let missing = persist(&harness).await;
    let old_running = persist(&harness).await;
    sqlx::query("UPDATE job_executions SET descriptor_json = '{invalid}' WHERE job_id = ?")
        .bind(invalid.id().to_string())
        .execute(&harness.pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM job_executions WHERE job_id = ?")
        .bind(missing.id().to_string())
        .execute(&harness.pool)
        .await
        .unwrap();
    let mut running = old_running.clone();
    running.start().unwrap();
    harness.store.start(&running).await.unwrap();
    let recovery = RecoverInterruptedStateUseCase::new(std::sync::Arc::new(
        adapters_storage::sqlite::recovery::SqliteRecoveryStorage::new(harness.pool.clone()),
    ))
    .with_execution_store(harness.store.clone());
    let report = recovery.execute().await.unwrap();
    assert!(!report.has_blocking_failures());
    assert_eq!(report.actions_applied, 3);
    for (job, code) in [
        (invalid, "EXECUTION_DESCRIPTOR_INVALID"),
        (missing, "EXECUTION_DESCRIPTOR_MISSING"),
        (old_running, "APP_RESTART"),
    ] {
        let recovered = harness.jobs.get(job.id()).await.unwrap().unwrap();
        assert_eq!(recovered.status(), &JobStatus::Failed);
        assert_eq!(recovered.error().unwrap().code, code);
    }
    let events = harness.controlled().await;
    assert!(events.is_empty());
    harness.shutdown().await;
}
