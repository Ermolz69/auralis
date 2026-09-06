use std::sync::Arc;

use domain::job::Job;
use domain::project::Project;
use ports::job_runtime_control::{JobRuntimeControlPort, RuntimeCompletion};
use ports::job_scheduler::{JobSchedulerPort, ScheduledJob};
use ports::repository::ProjectRepository;
use ports::source::SubtitleSourcePort;
use ports::storage::ArtifactStore;
use ports::transaction::StorageUnitOfWork;

use super::mock_dubbing_pipeline::MockDubbingPipelineRunner;
use super::start_mock_compensation::compensate_start_failure;
use super::start_mock_runtime::{
    MockPipelineTaskSpec, runtime_start_handshake, spawn_mock_pipeline_task,
};
use crate::error::ApplicationError;

pub(super) struct MockPipelineRuntimeStarter {
    job_scheduler: Arc<dyn JobSchedulerPort>,
    storage_uow: Arc<dyn StorageUnitOfWork>,
    job_runtime: Arc<dyn JobRuntimeControlPort>,
}

impl MockPipelineRuntimeStarter {
    pub(super) fn new(
        job_scheduler: Arc<dyn JobSchedulerPort>,
        storage_uow: Arc<dyn StorageUnitOfWork>,
        job_runtime: Arc<dyn JobRuntimeControlPort>,
    ) -> Self {
        Self {
            job_scheduler,
            storage_uow,
            job_runtime,
        }
    }

    pub(super) async fn start<
        R: ProjectRepository + Clone + 'static,
        V: SubtitleSourcePort + Clone + 'static,
        T: StorageUnitOfWork + Clone + 'static,
        S: ArtifactStore + Clone + 'static,
    >(
        &self,
        runner: MockDubbingPipelineRunner<R, V, T, S>,
        project: &Project,
        job: &Job,
    ) -> Result<ScheduledJob, ApplicationError> {
        let job_id = job.id().clone();
        let project_id = project.id().clone();

        if let Err(error) = self
            .job_runtime
            .reserve(job_id.clone(), project_id.clone())
            .await
        {
            compensate_start_failure(self.storage_uow.clone(), project, job, error.to_string())
                .await?;
            return Err(ApplicationError::InvalidOperation {
                message: format!("Failed to reserve runtime: {error}"),
            });
        }

        let (cancel_handle, token) = ports::cancellation::CancelHandle::new();
        let completion = Arc::new(RuntimeCompletion::new());
        let (control, gates) = runtime_start_handshake();
        let span = tracing::info_span!(
            "job_execution",
            job_id = %job_id,
            project_id = %project_id,
            action = "job_execution"
        );
        let runtime_task = spawn_mock_pipeline_task(MockPipelineTaskSpec {
            runner,
            job_scheduler: self.job_scheduler.clone(),
            job_runtime: self.job_runtime.clone(),
            job_id: job_id.clone(),
            project_id,
            cancel_handle,
            token,
            completion,
            gates,
            span,
        });

        if let Err(error) = self
            .job_runtime
            .attach_task(job_id.clone(), runtime_task)
            .await
        {
            error.task.cancel.cancel();
            error.task.join_handle.abort();
            let join_result = error.task.join_handle.await;
            self.job_runtime.finish_now(&job_id);
            compensate_start_failure(
                self.storage_uow.clone(),
                project,
                job,
                error.source.to_string(),
            )
            .await?;
            if let Err(join_error) = join_result {
                tracing::warn!("runtime task aborted during attach rollback: {join_error}");
            }
            return Err(ApplicationError::InvalidOperation {
                message: "Failed to attach task".into(),
            });
        }

        let scheduled_job = match self.job_scheduler.enqueue_existing_job(&job_id).await {
            Ok(scheduled) => scheduled,
            Err(error) => return self.fail_scheduling(&job_id, project, job, error).await,
        };

        if control.activate.send(()).is_err() {
            return self.fail_handshake(&job_id, project, job, "activate").await;
        }
        if control.acknowledged.await.is_err() {
            return self.fail_handshake(&job_id, project, job, "ack").await;
        }
        if control.release.send(()).is_err() {
            return self.fail_handshake(&job_id, project, job, "release").await;
        }

        Ok(scheduled_job)
    }

    async fn fail_scheduling(
        &self,
        job_id: &domain::job::JobId,
        project: &Project,
        job: &Job,
        error: ports::error::PortError,
    ) -> Result<ScheduledJob, ApplicationError> {
        self.job_runtime.rollback_runtime_start(job_id).await?;
        if let Err(compensation_error) =
            compensate_start_failure(self.storage_uow.clone(), project, job, error.to_string())
                .await
        {
            return Err(ApplicationError::PipelineStartFailedNeedsRecovery {
                scheduling_error: error.to_string(),
                compensation_error: compensation_error.to_string(),
            });
        }
        Err(ApplicationError::PipelineStartFailed {
            scheduling_error: error.to_string(),
        })
    }

    async fn fail_handshake(
        &self,
        job_id: &domain::job::JobId,
        project: &Project,
        job: &Job,
        phase: &str,
    ) -> Result<ScheduledJob, ApplicationError> {
        self.job_runtime.rollback_runtime_start(job_id).await?;
        compensate_start_failure(
            self.storage_uow.clone(),
            project,
            job,
            format!("Failed to {phase}"),
        )
        .await?;
        Err(ApplicationError::InvalidOperation {
            message: format!("Failed to {phase} task"),
        })
    }
}
