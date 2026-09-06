use std::sync::Arc;

use adapters_storage::{local::LocalTempWorkspace, memory::InMemoryProjectRepository};
use async_trait::async_trait;
use ports::error::PortError;
use ports::job_runtime_control::{
    AttachTaskError, JobRuntimeControlPort, RuntimeCleanupOutcome, RuntimeCleanupReport,
    RuntimeTask,
};
use ports::job_scheduler::JobSchedulerPort;
use ports::source::{DownloadSubtitleRequest, SubtitleSourcePort};
use ports::transaction::StorageUnitOfWork;

use super::start_mock::{StartMockPipelineDependencies, StartMockPipelineUseCase};
use crate::test_utils::MockArtifactStore;

#[derive(Clone)]
pub(super) struct MockSubtitleSource;

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
        _request: DownloadSubtitleRequest,
    ) -> Result<domain::media::Artifact, PortError> {
        Err(PortError::Unsupported {
            message: "Not implemented".into(),
        })
    }
}

pub(super) fn start_use_case(
    project_repo: InMemoryProjectRepository,
    job_scheduler: Arc<dyn JobSchedulerPort>,
    storage_uow: Arc<dyn StorageUnitOfWork>,
) -> StartMockPipelineUseCase<InMemoryProjectRepository, MockSubtitleSource, MockArtifactStore> {
    StartMockPipelineUseCase::new(StartMockPipelineDependencies {
        project_repo,
        job_scheduler,
        storage_uow,
        subtitle_source: MockSubtitleSource,
        artifact_store: MockArtifactStore,
        workspace_port: Arc::new(LocalTempWorkspace::new(std::env::temp_dir())),
        locks: Arc::new(crate::usecases::project::lifecycle::ProjectLifecycleLocks::new()),
        job_runtime: Arc::new(MockJobRuntimeControl),
    })
}

struct MockJobRuntimeControl;

#[async_trait]
impl JobRuntimeControlPort for MockJobRuntimeControl {
    async fn cancel_and_evict_jobs(
        &self,
        _job_ids: &[domain::job::JobId],
    ) -> Result<RuntimeCleanupReport, PortError> {
        Ok(RuntimeCleanupReport {
            jobs: std::collections::HashMap::new(),
        })
    }

    async fn reserve(
        &self,
        _job_id: domain::job::JobId,
        _project_id: domain::project::ProjectId,
    ) -> Result<(), PortError> {
        Ok(())
    }

    async fn attach_task(
        &self,
        _job_id: domain::job::JobId,
        _task: RuntimeTask,
    ) -> Result<(), AttachTaskError> {
        Ok(())
    }

    fn finish_now(&self, _job_id: &domain::job::JobId) {}

    async fn rollback_runtime_start(
        &self,
        _job_id: &domain::job::JobId,
    ) -> Result<RuntimeCleanupOutcome, PortError> {
        Ok(RuntimeCleanupOutcome::ReservationRemoved)
    }
}
