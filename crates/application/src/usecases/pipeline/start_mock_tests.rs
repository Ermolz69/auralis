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
        _request: ports::source::DownloadSubtitleRequest,
    ) -> Result<domain::media::Artifact, PortError> {
        Err(PortError::Unsupported {
            message: "Not implemented".into(),
        })
    }
}

use crate::test_utils::MockArtifactStore;
use adapters_storage::local::LocalTempWorkspace;

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
async fn test_success_saves_project_processing_and_job_pending_running() {
    let project_repo = InMemoryProjectRepository::new(std::sync::Arc::new(std::sync::Mutex::new(
        adapters_storage::memory::InMemoryDatabase::new(),
    )));
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
        Arc::new(LocalTempWorkspace::new(std::path::PathBuf::from("/tmp"))),
        Arc::new(crate::usecases::project::lifecycle::ProjectLifecycleLocks::new()),
        Arc::new(MockJobRuntimeControl),
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
    let project_repo = InMemoryProjectRepository::new(std::sync::Arc::new(std::sync::Mutex::new(
        adapters_storage::memory::InMemoryDatabase::new(),
    )));
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
        Arc::new(LocalTempWorkspace::new(std::path::PathBuf::from("/tmp"))),
        Arc::new(crate::usecases::project::lifecycle::ProjectLifecycleLocks::new()),
        Arc::new(MockJobRuntimeControl),
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
    let project_repo = InMemoryProjectRepository::new(std::sync::Arc::new(std::sync::Mutex::new(
        adapters_storage::memory::InMemoryDatabase::new(),
    )));
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
        Arc::new(LocalTempWorkspace::new(std::path::PathBuf::from("/tmp"))),
        Arc::new(crate::usecases::project::lifecycle::ProjectLifecycleLocks::new()),
        Arc::new(MockJobRuntimeControl),
    );
    let request = StartMockPipelineRequest {
        project_id: project.id().clone(),
    };

    let response = use_case.execute(request).await;
    assert!(matches!(response, Err(ApplicationError::Domain { .. })));
}

#[tokio::test]
async fn test_cannot_start_from_completed() {
    let project_repo = InMemoryProjectRepository::new(std::sync::Arc::new(std::sync::Mutex::new(
        adapters_storage::memory::InMemoryDatabase::new(),
    )));
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
        Arc::new(LocalTempWorkspace::new(std::path::PathBuf::from("/tmp"))),
        Arc::new(crate::usecases::project::lifecycle::ProjectLifecycleLocks::new()),
        Arc::new(MockJobRuntimeControl),
    );
    let request = StartMockPipelineRequest {
        project_id: project.id().clone(),
    };

    let response = use_case.execute(request).await;
    assert!(matches!(response, Err(ApplicationError::Domain { .. })));
}
