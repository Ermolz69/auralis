use domain::project::{Project, ProjectId};
use ports::job_scheduler::{JobSchedulerPort, ScheduledJob};
use ports::repository::ProjectRepository;
use ports::transaction::{CommitPipelineStart, CommitPipelineStartFailure, StorageUnitOfWork};
use std::sync::Arc;

use crate::error::ApplicationError;
use crate::usecases::pipeline::mock_dubbing_pipeline::MockDubbingPipelineRunner;
use ports::source::SubtitleSourcePort;
use ports::storage::ArtifactStore;

#[derive(Debug)]
pub struct StartMockPipelineRequest {
    pub project_id: ProjectId,
}

#[derive(Debug)]
pub struct StartMockPipelineResponse {
    pub project: Project,
    pub job: ScheduledJob,
}

pub struct StartMockPipelineUseCase<
    R: ProjectRepository + Clone + 'static,
    V: SubtitleSourcePort + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> {
    project_repo: R,
    job_scheduler: Arc<dyn JobSchedulerPort>,
    storage_uow: Arc<dyn StorageUnitOfWork>,
    subtitle_source: V,
    artifact_store: S,
    target_dir_base: std::path::PathBuf,
}

impl<
    R: ProjectRepository + Clone + 'static,
    V: SubtitleSourcePort + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> StartMockPipelineUseCase<R, V, S>
{
    pub fn new(
        project_repo: R,
        job_scheduler: Arc<dyn JobSchedulerPort>,
        storage_uow: Arc<dyn StorageUnitOfWork>,
        subtitle_source: V,

        artifact_store: S,
        target_dir_base: std::path::PathBuf,
    ) -> Self {
        Self {
            project_repo,
            job_scheduler,
            storage_uow,
            subtitle_source,

            artifact_store,
            target_dir_base,
        }
    }

    pub async fn execute(
        &self,
        request: StartMockPipelineRequest,
    ) -> Result<StartMockPipelineResponse, ApplicationError> {
        let mut project = self
            .project_repo
            .get(&request.project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(request.project_id.clone()))?;

        // 1. Create Job (it starts as Pending)
        let job = domain::job::Job::new(
            project.id().clone(),
            project.title().to_string(),
            domain::job::JobKind::Dubbing,
        );

        let job_id = job.id().clone();

        // 2. Validate transition
        project.start_processing(job_id.clone())?;

        let commit_cmd = CommitPipelineStart {
            project: project.clone(),
            job: job.clone(),
        };
        self.storage_uow
            .commit_pipeline_start(commit_cmd)
            .await
            .map_err(|e| ApplicationError::InvalidOperation {
                message: format!("Failed to commit pipeline start: {}", e),
            })?;

        // 4. Enqueue the persisted job for asynchronous processing
        let enqueue_result = self.job_scheduler.enqueue_existing_job(&job_id).await;

        let job = match enqueue_result {
            Ok(scheduled) => scheduled,
            Err(enqueue_err) => {
                // Compensation
                let mut failed_project = project.clone();
                let mut failed_job = job.clone();

                let _ = failed_project.apply_terminal_transition(
                    failed_job.id(),
                    domain::job::TerminalOutcome::Failed,
                ); // Ignore transition error if any
                let _ = failed_job.mark_failed(domain::job::JobError::new(
                    "SCHEDULING_FAILED",
                    format!("Failed to schedule job: {}", enqueue_err),
                    false,
                ));

                let failure_cmd = CommitPipelineStartFailure {
                    project: failed_project,
                    job: failed_job,
                };

                match self
                    .storage_uow
                    .commit_pipeline_start_failure(failure_cmd)
                    .await
                {
                    Ok(_) => {
                        return Err(ApplicationError::PipelineStartFailed {
                            scheduling_error: enqueue_err.to_string(),
                        });
                    }
                    Err(comp_err) => {
                        return Err(ApplicationError::PipelineStartFailedNeedsRecovery {
                            scheduling_error: enqueue_err.to_string(),
                            compensation_error: comp_err.to_string(),
                        });
                    }
                }
            }
        };

        // 5. Spawn the mock pipeline runner
        let runner = MockDubbingPipelineRunner::new(
            self.job_scheduler.clone(),
            self.project_repo.clone(),
            self.subtitle_source.clone(),
            self.storage_uow.clone(),
            self.artifact_store.clone(),
            self.target_dir_base.clone(),
        );

        runner.spawn(job.id.clone(), request.project_id.clone());

        Ok(StartMockPipelineResponse { project, job })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{MockJobScheduler, MockStorageUnitOfWork};
    use adapters_storage::memory::InMemoryProjectRepository;
    use async_trait::async_trait;
    use domain::job::JobStatus;

    use ports::error::PortError;

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
            _source: &domain::media::MediaSource,
            _track: &domain::media::SubtitleTrack,
            _target_path: &std::path::Path,
        ) -> Result<domain::media::Artifact, PortError> {
            Err(PortError::Unsupported {
                message: "Not implemented".into(),
            })
        }
    }

    use crate::test_utils::MockArtifactStore;

    #[tokio::test]
    async fn test_success_saves_project_processing_and_job_pending_running() {
        let project_repo = InMemoryProjectRepository::new();
        let job_scheduler = Arc::new(MockJobScheduler::new());
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
            std::path::PathBuf::from("/tmp"),
        );
        let request = StartMockPipelineRequest {
            project_id: project.id().clone(),
        };

        let response = use_case.execute(request).await.unwrap();

        assert_eq!(response.job.status, JobStatus::Running); // MockJobScheduler sets to Running

        // Verify project in the database via the mock uow directly,
        // since the in-memory adapter saves via repository
        let projects_saved = tx_gateway.projects_saved.lock().await;
        assert_eq!(projects_saved.len(), 1);
        assert_eq!(
            projects_saved[0].status(),
            &domain::project::ProjectStatus::Processing
        );

        let jobs_saved = tx_gateway.jobs_saved.lock().await;
        assert_eq!(jobs_saved.len(), 1);
        assert_eq!(jobs_saved[0].status(), &JobStatus::Pending); // The usecase saves as pending!
    }

    #[tokio::test]
    async fn test_transaction_failure_does_not_enqueue_job() {
        let project_repo = InMemoryProjectRepository::new();
        let job_scheduler = Arc::new(MockJobScheduler::new());
        let tx_gateway = Arc::new(MockStorageUnitOfWork::with_failure()); // Will fail

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
            std::path::PathBuf::from("/tmp"),
        );
        let request = StartMockPipelineRequest {
            project_id: project.id().clone(),
        };

        let response = use_case.execute(request).await;
        assert!(matches!(
            response,
            Err(ApplicationError::InvalidOperation { .. })
        ));

        // Verify job was NOT enqueued
        let scheduled_jobs = job_scheduler.jobs.lock().await;
        assert_eq!(scheduled_jobs.len(), 0);
    }

    #[tokio::test]
    async fn test_cannot_start_from_draft() {
        let project_repo = InMemoryProjectRepository::new();
        let job_scheduler = Arc::new(MockJobScheduler::new());
        let tx_gateway = Arc::new(MockStorageUnitOfWork::new());

        let project = Project::new("Test".to_string());
        project_repo.create(project.clone()).await.unwrap();

        let use_case = StartMockPipelineUseCase::new(
            project_repo.clone(),
            job_scheduler.clone(),
            tx_gateway.clone(),
            MockSubtitleSource,
            MockArtifactStore,
            std::path::PathBuf::from("/tmp"),
        );
        let request = StartMockPipelineRequest {
            project_id: project.id().clone(),
        };

        let response = use_case.execute(request).await;
        assert!(matches!(response, Err(ApplicationError::Domain { .. })));
    }

    #[tokio::test]
    async fn test_cannot_start_from_completed() {
        let project_repo = InMemoryProjectRepository::new();
        let job_scheduler = Arc::new(MockJobScheduler::new());
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
        let test_job_id = domain::job::JobId::new();
        project.start_processing(test_job_id.clone()).unwrap();
        project
            .apply_terminal_transition(&test_job_id, domain::job::TerminalOutcome::Completed)
            .unwrap();
        project_repo.create(project.clone()).await.unwrap();

        let use_case = StartMockPipelineUseCase::new(
            project_repo.clone(),
            job_scheduler.clone(),
            tx_gateway.clone(),
            MockSubtitleSource,
            MockArtifactStore,
            std::path::PathBuf::from("/tmp"),
        );
        let request = StartMockPipelineRequest {
            project_id: project.id().clone(),
        };

        let response = use_case.execute(request).await;
        assert!(matches!(response, Err(ApplicationError::Domain { .. })));
    }

    #[tokio::test]
    async fn test_enqueue_failure_compensates_and_marks_failed() {
        let project_repo = InMemoryProjectRepository::new();
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
            std::path::PathBuf::from("/tmp"),
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
        let project_repo = InMemoryProjectRepository::new();
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
            ) -> Result<(), PortError> {
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
                Ok(domain::project::status::TerminalTransitionResult::Applied)
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
            std::path::PathBuf::from("/tmp"),
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
}
