use super::support::*;
#[path = "../support/queue_subtitles.rs"]
mod subtitles;
use application::usecases::pipeline::start_mock::StartMockPipelineRequest;
use domain::job::JobStatus;
use jobs::queue::JobQueueConfig;
use ports::{job_resources::ResourceDemand, job_scheduler::JobSchedulerPort};
use std::{sync::Arc, time::Duration};

#[tokio::test]
async fn real_application_admission_dispatch_and_import_preserve_selected_track() {
    let harness = Harness::new(JobQueueConfig {
        budgets: ResourceDemand {
            cpu: 2,
            network: 2,
            disk: 2,
            processes: 2,
        },
        ..Default::default()
    })
    .await;
    let (send, mut downloads) = tokio::sync::mpsc::unbounded_channel();
    let pipeline = harness.pipeline(subtitles::FixtureSubtitles(send), harness.workspace());
    harness
        .manager
        .start_dispatcher(pipeline.queued_executor())
        .await
        .unwrap();
    let first = harness.project().await;
    let response = pipeline
        .execute(StartMockPipelineRequest {
            project_id: first.id().clone(),
            selected_subtitle_track: Some(subtitles::track()),
        })
        .await
        .unwrap();
    assert_eq!(response.job.status, JobStatus::Pending);
    let first_download = subtitles::downloaded(&mut downloads).await;
    let second = harness.project().await;
    let response2 = pipeline
        .execute(StartMockPipelineRequest {
            project_id: second.id().clone(),
            selected_subtitle_track: None,
        })
        .await
        .unwrap();
    let second_download = subtitles::downloaded(&mut downloads).await;
    assert_eq!(harness.status(&response.job.id).await, JobStatus::Running);
    assert_eq!(harness.status(&response2.job.id).await, JobStatus::Running);
    assert_eq!(first_download.track, subtitles::track());
    assert!(
        pipeline
            .execute(StartMockPipelineRequest {
                project_id: first.id().clone(),
                selected_subtitle_track: None
            })
            .await
            .is_err()
    );
    let third = harness.project().await;
    let response3 = pipeline
        .execute(StartMockPipelineRequest {
            project_id: third.id().clone(),
            selected_subtitle_track: None,
        })
        .await
        .unwrap();
    assert_eq!(harness.status(&response3.job.id).await, JobStatus::Pending);
    first_download.release.send(true).unwrap();
    let third_download = tokio::time::timeout(Duration::from_secs(5), downloads.recv())
        .await
        .unwrap()
        .unwrap();
    second_download.release.send(true).unwrap();
    third_download.release.send(true).unwrap();
    harness
        .wait_status(&response3.job.id, JobStatus::Completed)
        .await;
    use ports::repository::ProjectRepository;
    assert!(
        harness
            .projects
            .get(third.id())
            .await
            .unwrap()
            .unwrap()
            .transcript()
            .is_some()
    );
    harness.shutdown().await;
}

#[tokio::test]
async fn outer_cancel_does_not_drop_import_workspace_cleanup() {
    let harness = Harness::new(JobQueueConfig {
        budgets: ResourceDemand {
            cpu: 1,
            network: 1,
            disk: 1,
            processes: 1,
        },
        ..Default::default()
    })
    .await;
    let (send, mut downloads) = tokio::sync::mpsc::unbounded_channel();
    let workspace = Arc::new(subtitles::CleanupWorkspace {
        inner: harness.workspace(),
        entered: Default::default(),
        release: Default::default(),
        fail: false,
    });
    let pipeline = harness.pipeline(subtitles::FixtureSubtitles(send), workspace.clone());
    harness
        .manager
        .start_dispatcher(pipeline.queued_executor())
        .await
        .unwrap();
    let project = harness.project().await;
    let response = pipeline
        .execute(StartMockPipelineRequest {
            project_id: project.id().clone(),
            selected_subtitle_track: None,
        })
        .await
        .unwrap();
    let _download = subtitles::downloaded(&mut downloads).await;
    let manager = harness.manager.clone();
    let id = response.job.id.clone();
    let cancel = tokio::spawn(async move { manager.cancel_job(&id).await });
    tokio::time::timeout(Duration::from_secs(5), workspace.entered.cancelled())
        .await
        .unwrap();
    assert_eq!(
        harness.status(&response.job.id).await,
        JobStatus::Cancelling
    );
    assert!(!cancel.is_finished());
    assert_eq!(
        harness
            .manager
            .queue_available_resources()
            .unwrap()
            .processes,
        0
    );
    workspace.release.cancel();
    assert_eq!(cancel.await.unwrap().unwrap().status, JobStatus::Cancelled);
    assert_eq!(
        harness
            .manager
            .queue_available_resources()
            .unwrap()
            .processes,
        1
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM artifacts")
        .fetch_one(&harness.pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    harness.shutdown().await;
}

#[tokio::test]
async fn failed_cleanup_is_durable_and_quarantines_resources() {
    let harness = Harness::new(JobQueueConfig {
        budgets: ResourceDemand {
            cpu: 1,
            network: 1,
            disk: 1,
            processes: 1,
        },
        ..Default::default()
    })
    .await;
    let (send, mut downloads) = tokio::sync::mpsc::unbounded_channel();
    let workspace = Arc::new(subtitles::CleanupWorkspace {
        inner: harness.workspace(),
        entered: Default::default(),
        release: Default::default(),
        fail: true,
    });
    workspace.release.cancel();
    let pipeline = harness.pipeline(subtitles::FixtureSubtitles(send), workspace);
    harness
        .manager
        .start_dispatcher(pipeline.queued_executor())
        .await
        .unwrap();
    let project = harness.project().await;
    let response = pipeline
        .execute(StartMockPipelineRequest {
            project_id: project.id().clone(),
            selected_subtitle_track: None,
        })
        .await
        .unwrap();
    let _download = subtitles::downloaded(&mut downloads).await;
    assert!(harness.manager.cancel_job(&response.job.id).await.is_err());
    assert_eq!(
        harness.status(&response.job.id).await,
        JobStatus::Cancelling
    );
    let job = harness
        .manager
        .get_job(&response.job.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        job.progress.current_step.as_deref(),
        Some("cleanup_unconfirmed")
    );
    assert_eq!(harness.manager.queue_available_resources().unwrap().cpu, 0);
    assert!(harness.manager.cancel_job(&response.job.id).await.is_err());
    let report = ports::job_runtime_control::JobRuntimeControlPort::drain_all(
        harness.manager.as_ref(),
        Duration::from_secs(1),
    )
    .await
    .unwrap();
    assert_eq!(report.unconfirmed_count, 1);
}

#[tokio::test]
async fn successful_import_retains_resources_until_workspace_cleanup_finishes() {
    let harness = Harness::new(JobQueueConfig::default()).await;
    let (send, mut downloads) = tokio::sync::mpsc::unbounded_channel();
    let workspace = Arc::new(subtitles::CleanupWorkspace {
        inner: harness.workspace(),
        entered: Default::default(),
        release: Default::default(),
        fail: false,
    });
    let pipeline = harness.pipeline(subtitles::FixtureSubtitles(send), workspace.clone());
    harness
        .manager
        .start_dispatcher(pipeline.queued_executor())
        .await
        .unwrap();
    let project = harness.project().await;
    let response = pipeline
        .execute(StartMockPipelineRequest {
            project_id: project.id().clone(),
            selected_subtitle_track: None,
        })
        .await
        .unwrap();
    subtitles::downloaded(&mut downloads)
        .await
        .release
        .send(true)
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), workspace.entered.cancelled())
        .await
        .unwrap();
    assert_eq!(harness.status(&response.job.id).await, JobStatus::Running);
    assert_eq!(
        harness
            .manager
            .queue_available_resources()
            .unwrap()
            .processes,
        1
    );
    workspace.release.cancel();
    harness
        .wait_status(&response.job.id, JobStatus::Completed)
        .await;
    harness.shutdown().await;
}
