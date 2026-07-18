use domain::project::{Project, ProjectId};
use ports::job_scheduler::{JobSchedulerPort, ScheduledJob};
use ports::repository::ProjectRepository;
use ports::transaction::{CommitPipelineStart, StorageUnitOfWork};
use std::sync::Arc;

use crate::error::ApplicationError;
use crate::usecases::pipeline::mock_dubbing_pipeline::MockDubbingPipelineRunner;
use crate::usecases::pipeline::start_mock_compensation::compensate_start_failure;
use crate::usecases::pipeline::start_mock_runtime::spawn_mock_pipeline_task;
use ports::source::SubtitleSourcePort;
use ports::storage::ArtifactStore;
use ports::workspace::TempWorkspacePort;

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
    workspace_port: Arc<dyn TempWorkspacePort>,
    locks: Arc<crate::usecases::project::lifecycle::ProjectLifecycleLocks>,
    job_runtime: Arc<dyn ports::job_runtime_control::JobRuntimeControlPort>,
}

impl<
    R: ProjectRepository + Clone + 'static,
    V: SubtitleSourcePort + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> StartMockPipelineUseCase<R, V, S>
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_repo: R,
        job_scheduler: Arc<dyn JobSchedulerPort>,
        storage_uow: Arc<dyn StorageUnitOfWork>,
        subtitle_source: V,
        artifact_store: S,
        workspace_port: Arc<dyn TempWorkspacePort>,
        locks: Arc<crate::usecases::project::lifecycle::ProjectLifecycleLocks>,
        job_runtime: Arc<dyn ports::job_runtime_control::JobRuntimeControlPort>,
    ) -> Self {
        Self {
            project_repo,
            job_scheduler,
            storage_uow,
            subtitle_source,
            artifact_store,
            workspace_port,
            locks,
            job_runtime,
        }
    }

    pub async fn execute(
        &self,
        request: StartMockPipelineRequest,
    ) -> Result<StartMockPipelineResponse, ApplicationError> {
        // 1. Acquire ProjectLifecycleLock
        let lock_arc = self.locks.get_lock(&request.project_id)?;
        let _lock = lock_arc.lock().await;

        let mut project = self
            .project_repo
            .get(&request.project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(request.project_id.clone()))?;

        // Create Job
        let job = domain::job::Job::new(
            project.id().clone(),
            project.title().to_string(),
            domain::job::JobKind::Dubbing,
        );
        let job_id = job.id().clone();
        project.start_processing(job_id.clone())?;

        // 2. Commit Pending job to DB
        let commit_cmd = CommitPipelineStart {
            project: project.clone(),
            job: job.clone(),
        };
        self.storage_uow
            .commit_pipeline_start(commit_cmd)
            .await
            .map_err(|e| ApplicationError::InvalidOperation {
                message: format!("Failed to commit pipeline start: {}", e),
            })?; // If failed -> no runtime registration

        // 3. job_runtime.reserve()
        if let Err(e) = self
            .job_runtime
            .reserve(job_id.clone(), request.project_id.clone())
            .await
        {
            compensate_start_failure(self.storage_uow.clone(), &project, &job, e.to_string())
                .await?;
            return Err(ApplicationError::InvalidOperation {
                message: format!("Failed to reserve runtime: {}", e),
            });
        }

        // 4. Create tokens, Completion, and Two-Phase Gates
        let (cancel_handle, token) = ports::cancellation::CancelHandle::new();
        let completion = Arc::new(ports::job_runtime_control::RuntimeCompletion::new());
        let (activate_tx, activate_rx) = tokio::sync::oneshot::channel::<()>();
        let (ack_tx, ack_rx) = tokio::sync::oneshot::channel::<()>();
        let (release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();

        let runner = MockDubbingPipelineRunner::new(
            self.job_scheduler.clone(),
            self.project_repo.clone(),
            self.subtitle_source.clone(),
            self.storage_uow.clone(),
            self.artifact_store.clone(),
            self.workspace_port.clone(),
            self.job_runtime.clone(),
        );

        let span = tracing::info_span!("job_execution", job_id = %job_id, project_id = %request.project_id, action = "job_execution");

        let runtime_task = spawn_mock_pipeline_task(
            runner,
            self.job_scheduler.clone(),
            self.job_runtime.clone(),
            job_id.clone(),
            request.project_id.clone(),
            cancel_handle.clone(),
            token,
            completion,
            activate_rx,
            ack_tx,
            release_rx,
            span,
        );

        if let Err(e) = self
            .job_runtime
            .attach_task(job_id.clone(), runtime_task)
            .await
        {
            // Take RuntimeTask back, abort, await, remove reservation, apply compensation
            e.task.cancel.cancel();
            e.task.join_handle.abort();
            let join_result = e.task.join_handle.await;
            self.job_runtime.finish_now(&job_id); // remove reservation
            compensate_start_failure(
                self.storage_uow.clone(),
                &project,
                &job,
                e.source.to_string(),
            )
            .await?;
            if let Err(join_err) = join_result {
                tracing::warn!("runtime task aborted during attach rollback: {}", join_err);
            }
            return Err(ApplicationError::InvalidOperation {
                message: "Failed to attach task".into(),
            });
        }

        // 8. Atomic Commit
        let enqueue_result = self.job_scheduler.enqueue_existing_job(&job_id).await;
        let scheduled_job = match enqueue_result {
            Ok(scheduled) => scheduled,
            Err(enqueue_err) => {
                // rollback_runtime_start, abort, await, compensation
                self.job_runtime.rollback_runtime_start(&job_id).await?;
                if let Err(comp_err) = compensate_start_failure(
                    self.storage_uow.clone(),
                    &project,
                    &job,
                    enqueue_err.to_string(),
                )
                .await
                {
                    return Err(ApplicationError::PipelineStartFailedNeedsRecovery {
                        scheduling_error: enqueue_err.to_string(),
                        compensation_error: comp_err.to_string(),
                    });
                }
                return Err(ApplicationError::PipelineStartFailed {
                    scheduling_error: enqueue_err.to_string(),
                });
            }
        };

        // 9. Send activate and await ack
        if activate_tx.send(()).is_err() {
            self.job_runtime.rollback_runtime_start(&job_id).await?;
            compensate_start_failure(
                self.storage_uow.clone(),
                &project,
                &job,
                "Failed to activate".into(),
            )
            .await?;
            return Err(ApplicationError::InvalidOperation {
                message: "Failed to activate task".into(),
            });
        }

        if ack_rx.await.is_err() {
            self.job_runtime.rollback_runtime_start(&job_id).await?;
            compensate_start_failure(
                self.storage_uow.clone(),
                &project,
                &job,
                "Failed to ack".into(),
            )
            .await?;
            return Err(ApplicationError::InvalidOperation {
                message: "Failed to ack task".into(),
            });
        }

        // Event publication
        // Currently JobManager publishes events on enqueue_existing_job.

        // Open start barrier
        if release_tx.send(()).is_err() {
            self.job_runtime.rollback_runtime_start(&job_id).await?;
            compensate_start_failure(
                self.storage_uow.clone(),
                &project,
                &job,
                "Failed to release".into(),
            )
            .await?;
            return Err(ApplicationError::InvalidOperation {
                message: "Failed to release task".into(),
            });
        }

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
