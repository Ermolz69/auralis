use domain::project::{Project, ProjectId};
use ports::job_scheduler::{JobSchedulerPort, ScheduledJob};
use ports::repository::ProjectRepository;
use ports::transaction::{CommitPipelineStart, StorageUnitOfWork};
use std::sync::Arc;

use crate::error::ApplicationError;
use crate::usecases::pipeline::mock_dubbing_pipeline::MockDubbingPipelineRunner;
use crate::usecases::pipeline::start_mock_activation::MockPipelineRuntimeStarter;
use ports::source::SubtitleSourcePort;
use ports::storage::ArtifactStore;
use ports::workspace::TempWorkspacePort;

#[derive(Debug)]
pub struct StartMockPipelineRequest {
    pub project_id: ProjectId,
    pub selected_subtitle_track: Option<domain::media::SubtitleTrack>,
}

#[derive(Debug)]
pub struct StartMockPipelineResponse {
    pub project: Project,
    pub job: ScheduledJob,
}

pub struct StartMockPipelineDependencies<
    R: ProjectRepository + Clone + 'static,
    V: SubtitleSourcePort + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> {
    pub project_repo: R,
    pub job_scheduler: Arc<dyn JobSchedulerPort>,
    pub storage_uow: Arc<dyn StorageUnitOfWork>,
    pub subtitle_source: V,
    pub artifact_store: S,
    pub workspace_port: Arc<dyn TempWorkspacePort>,
    pub locks: Arc<crate::usecases::project::lifecycle::ProjectLifecycleLocks>,
    pub job_runtime: Arc<dyn ports::job_runtime_control::JobRuntimeControlPort>,
}

pub struct StartMockPipelineUseCase<
    R: ProjectRepository + Clone + 'static,
    V: SubtitleSourcePort + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> {
    dependencies: StartMockPipelineDependencies<R, V, S>,
}

impl<
    R: ProjectRepository + Clone + 'static,
    V: SubtitleSourcePort + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> StartMockPipelineUseCase<R, V, S>
{
    pub fn new(dependencies: StartMockPipelineDependencies<R, V, S>) -> Self {
        Self { dependencies }
    }

    pub async fn execute(
        &self,
        request: StartMockPipelineRequest,
    ) -> Result<StartMockPipelineResponse, ApplicationError> {
        let dependencies = &self.dependencies;
        let lock_arc = dependencies.locks.get_lock(&request.project_id)?;
        let _lock = lock_arc.lock().await;

        let mut project = dependencies
            .project_repo
            .get(&request.project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(request.project_id.clone()))?;

        let job = domain::job::Job::new(
            project.id().clone(),
            project.title().to_string(),
            domain::job::JobKind::Dubbing,
        );
        let job_id = job.id().clone();
        project.start_processing(job_id.clone())?;

        dependencies
            .storage_uow
            .commit_pipeline_start(CommitPipelineStart {
                project: project.clone(),
                job: job.clone(),
            })
            .await
            .map_err(|e| ApplicationError::InvalidOperation {
                message: format!("Failed to commit pipeline start: {}", e),
            })?;
        project.advance_revision()?;

        let runner = MockDubbingPipelineRunner::new(
            dependencies.job_scheduler.clone(),
            dependencies.project_repo.clone(),
            dependencies.subtitle_source.clone(),
            dependencies.storage_uow.clone(),
            dependencies.artifact_store.clone(),
            dependencies.workspace_port.clone(),
            dependencies.job_runtime.clone(),
        )
        .with_selected_subtitle_track(request.selected_subtitle_track);

        let runtime_starter = MockPipelineRuntimeStarter::new(
            dependencies.job_scheduler.clone(),
            dependencies.storage_uow.clone(),
            dependencies.job_runtime.clone(),
        );
        let scheduled_job = runtime_starter.start(runner, &project, &job).await?;

        Ok(StartMockPipelineResponse {
            project,
            job: scheduled_job,
        })
    }
}

#[cfg(test)]
#[path = "start_mock_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "start_mock_compensation_tests.rs"]
mod compensation_tests;
