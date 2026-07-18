use super::*;
use crate::test_utils::{MockArtifactStore, MockJobScheduler, MockStorageUnitOfWork};
use adapters_storage::{local::LocalTempWorkspace, memory::InMemoryProjectRepository};
use async_trait::async_trait;
use domain::job::JobStatus;
use ports::error::PortError;
use std::sync::Arc;

#[derive(Clone)]
struct MockSubtitleSource;

#[async_trait]
impl SubtitleSourcePort for MockSubtitleSource {
    async fn list_subtitles(
        &self,
        _source: &domain::media::MediaSource,
    ) -> Result<Vec<domain::media::SubtitleTrack>, PortError> {
        Ok(vec![])
    }

    async fn download_subtitle(
        &self,
        _request: ports::source::DownloadSubtitleRequest,
    ) -> Result<domain::media::Artifact, PortError> {
        Err(PortError::Unsupported {
            message: "Not implemented".into(),
        })
    }
}

struct MockJobRuntimeControl;

#[async_trait::async_trait]
impl ports::job_runtime_control::JobRuntimeControlPort for MockJobRuntimeControl {
    async fn cancel_and_evict_jobs(
        &self,
        _job_ids: &[domain::job::JobId],
    ) -> Result<ports::job_runtime_control::RuntimeCleanupReport, ports::error::PortError> {
        Ok(ports::job_runtime_control::RuntimeCleanupReport {
            jobs: std::collections::HashMap::new(),
        })
    }

    async fn reserve(
        &self,
        _job_id: domain::job::JobId,
        _project_id: domain::project::ProjectId,
    ) -> Result<(), ports::error::PortError> {
        Ok(())
    }

    async fn attach_task(
        &self,
        _job_id: domain::job::JobId,
        _task: ports::job_runtime_control::RuntimeTask,
    ) -> Result<(), ports::job_runtime_control::AttachTaskError> {
        Ok(())
    }

    fn finish_now(&self, _job_id: &domain::job::JobId) {}

    async fn rollback_runtime_start(
        &self,
        _job_id: &domain::job::JobId,
    ) -> Result<ports::job_runtime_control::RuntimeCleanupOutcome, ports::error::PortError> {
        Ok(ports::job_runtime_control::RuntimeCleanupOutcome::ReservationRemoved)
    }
}

#[tokio::test]
async fn test_enqueue_failure_compensates_and_marks_failed() {
    let project_repo = InMemoryProjectRepository::new(std::sync::Arc::new(std::sync::Mutex::new(
        adapters_storage::memory::InMemoryDatabase::new(),
    )));
    let mut job_scheduler = MockJobScheduler::new();
    job_scheduler.should_fail = true; // Make enqueue fail
    let job_scheduler = Arc::new(job_scheduler);
    let tx_gateway = Arc::new(MockStorageUnitOfWork::new());

    let mut project = Project::new("Test".to_string());
    project
        .import_source(
            domain::media::MediaSource::RemoteUrl {
                url: "http://example.com".into(),
            },
            None,
        )
        .unwrap();
    project.mark_ready_for_processing().unwrap();
    project_repo.create(project.clone()).await.unwrap();

    let use_case = StartMockPipelineUseCase::new(
        project_repo.clone(),
        job_scheduler.clone(),
        tx_gateway.clone(),
        MockSubtitleSource,
        MockArtifactStore,
        Arc::new(LocalTempWorkspace::new(std::path::PathBuf::from("/tmp"))),
        Arc::new(crate::usecases::project::lifecycle::ProjectLifecycleLocks::new()),
        Arc::new(MockJobRuntimeControl),
    );
    let request = StartMockPipelineRequest {
        project_id: project.id().clone(),
    };

    let response = use_case.execute(request).await;

    match response {
        Err(ApplicationError::PipelineStartFailed { scheduling_error }) => {
            assert!(
                scheduling_error.contains("queue is full")
                    || scheduling_error.contains("Simulated")
                    || !scheduling_error.is_empty()
            );
        }
        _ => panic!("Expected enqueue failure error"),
    }

    // Verify compensation occurred
    let projects_saved = tx_gateway.projects_saved.lock().await;
    // 2 saves: initial start, and compensation
    assert_eq!(projects_saved.len(), 2);
    assert_eq!(
        projects_saved[1].status(),
        &domain::project::ProjectStatus::Failed
    );

    let jobs_saved = tx_gateway.jobs_saved.lock().await;
    assert_eq!(jobs_saved.len(), 2);
    assert_eq!(jobs_saved[1].status(), &JobStatus::Failed);
}

#[tokio::test]
async fn test_enqueue_and_compensation_failure_returns_both_errors() {
    let project_repo = InMemoryProjectRepository::new(std::sync::Arc::new(std::sync::Mutex::new(
        adapters_storage::memory::InMemoryDatabase::new(),
    )));
    let mut job_scheduler = MockJobScheduler::new();
    job_scheduler.should_fail = true; // Make enqueue fail
    let job_scheduler = Arc::new(job_scheduler);

    // Custom mock to fail only on compensation
    #[derive(Clone)]
    struct FailCompUow {
        inner: Arc<MockStorageUnitOfWork>,
    }
    #[async_trait]
    impl StorageUnitOfWork for FailCompUow {
        async fn commit_artifact_finalize(
            &self,
            cmd: ports::transaction::CommitArtifactFinalize,
        ) -> Result<ports::transaction::CommitArtifactFinalizeResult, PortError> {
            self.inner.commit_artifact_finalize(cmd).await
        }
        async fn commit_transcript_import(
            &self,
            cmd: ports::transaction::CommitTranscriptImport,
        ) -> Result<(), PortError> {
            self.inner.commit_transcript_import(cmd).await
        }
        async fn commit_staged_artifact_write(
            &self,
            cmd: ports::transaction::CommitStagedArtifactWrite,
        ) -> Result<(), PortError> {
            self.inner.commit_staged_artifact_write(cmd).await
        }

        async fn commit_managed_source_import(
            &self,
            cmd: ports::transaction::CommitManagedSourceImport,
        ) -> Result<(), PortError> {
            self.inner.commit_managed_source_import(cmd).await
        }
        async fn commit_project_delete(
            &self,
            cmd: ports::transaction::CommitProjectDelete,
        ) -> Result<ports::transaction::CommitProjectDeleteResult, PortError> {
            self.inner.commit_project_delete(cmd).await
        }
        async fn commit_job_update(
            &self,
            cmd: ports::transaction::CommitJobUpdate,
        ) -> Result<(), PortError> {
            self.inner.commit_job_update(cmd).await
        }
        async fn commit_pipeline_start(
            &self,
            cmd: ports::transaction::CommitPipelineStart,
        ) -> Result<(), PortError> {
            self.inner.commit_pipeline_start(cmd).await
        }
        async fn commit_pipeline_start_failure(
            &self,
            _command: ports::transaction::CommitPipelineStartFailure,
        ) -> Result<(), PortError> {
            Err(PortError::Unexpected {
                message: "UoW failed".into(),
            })
        }

        async fn commit_terminal_job_update(
            &self,
            _command: ports::transaction::CommitTerminalJobUpdate,
        ) -> Result<(), PortError> {
            Ok(())
        }

        async fn apply_terminal_lifecycle_conditionally(
            &self,
            _command: ports::transaction::ApplyTerminalLifecycle,
        ) -> Result<domain::project::status::TerminalTransitionResult, PortError> {
            Ok(domain::project::status::TerminalTransitionResult::Applied {
                transcript_ready: false,
            })
        }
    }

    let tx_gateway = Arc::new(FailCompUow {
        inner: Arc::new(MockStorageUnitOfWork::new()),
    });

    let mut project = Project::new("Test".to_string());
    project
        .import_source(
            domain::media::MediaSource::RemoteUrl {
                url: "http://example.com".into(),
            },
            None,
        )
        .unwrap();
    project.mark_ready_for_processing().unwrap();
    project_repo.create(project.clone()).await.unwrap();

    let use_case = StartMockPipelineUseCase::new(
        project_repo.clone(),
        job_scheduler.clone(),
        tx_gateway.clone(),
        MockSubtitleSource,
        MockArtifactStore,
        Arc::new(LocalTempWorkspace::new(std::path::PathBuf::from("/tmp"))),
        Arc::new(crate::usecases::project::lifecycle::ProjectLifecycleLocks::new()),
        Arc::new(MockJobRuntimeControl),
    );
    let request = StartMockPipelineRequest {
        project_id: project.id().clone(),
    };

    let response = use_case.execute(request).await;

    match response {
        Err(ApplicationError::PipelineStartFailedNeedsRecovery {
            scheduling_error,
            compensation_error,
        }) => {
            assert!(!scheduling_error.is_empty());
            assert!(compensation_error.contains("UoW failed"));
        }
        _ => panic!("Expected enqueue and compensation failure error"),
    }
}
