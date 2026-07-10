use domain::project::{Project, ProjectId};
use ports::job_scheduler::{JobSchedulerPort, ScheduledJob};
use ports::repository::ProjectRepository;
use ports::transaction::{CommitJobUpdate, StorageUnitOfWork};
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

        // 1. Validate transition
        project.mark_processing_started()?;

        // 2. Create Job (it starts as Pending)
        let job = domain::job::Job::new(
            project.id().clone(),
            project.title().to_string(),
            domain::job::JobKind::Dubbing,
        );

        let job_id = job.id().clone();

        // 3. Persist Project and Job changes
        self.project_repo.save(&project).await?;

        let commit_cmd = CommitJobUpdate { job: job.clone() };
        self.storage_uow
            .commit_job_update(commit_cmd)
            .await
            .map_err(|e| ApplicationError::InvalidOperation {
                message: format!("Failed to commit job update: {}", e),
            })?;

        // 4. Enqueue the persisted job for asynchronous processing
        let job = self
            .job_scheduler
            .enqueue_existing_job(&job_id)
            .await
            .map_err(|e| ApplicationError::InvalidOperation {
                message: format!("Failed to enqueue existing job: {}", e),
            })?;

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

        // Verify project_repo received the saved project
        let saved_project = project_repo.get(project.id()).await.unwrap().unwrap();
        assert_eq!(
            saved_project.status(),
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
        // Forcibly set to Completed is difficult without proper transitions, but we can try to do:
        project
            .import_source(
                domain::media::MediaSource::RemoteUrl {
                    url: "http://example.com".into(),
                },
                None,
            )
            .unwrap();
        project.mark_ready_for_processing().unwrap();
        project.mark_processing_started().unwrap();
        project.mark_completed().unwrap();
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
}
