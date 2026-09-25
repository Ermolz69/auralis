use super::{
    RunManagedTranslationUseCase, TranslationRunRequest, TranslationRunUseCase, managed_worker,
    pending_job_failure,
};
use crate::error::ApplicationError;
use domain::job::Job;
use ports::job_runtime_control::JobRuntimeControlPort;
use ports::translation_host_job::TranslationHostJobStore;
use std::sync::Arc;
use uuid::Uuid;

pub struct ScheduleManagedTranslationUseCase {
    managed: RunManagedTranslationUseCase,
    runtime: Arc<dyn JobRuntimeControlPort>,
    jobs: Arc<dyn TranslationHostJobStore>,
    pause: TranslationRunUseCase,
}

impl ScheduleManagedTranslationUseCase {
    pub fn new(
        managed: RunManagedTranslationUseCase,
        runtime: Arc<dyn JobRuntimeControlPort>,
        jobs: Arc<dyn TranslationHostJobStore>,
        pause: TranslationRunUseCase,
    ) -> Self {
        Self {
            managed,
            runtime,
            jobs,
            pause,
        }
    }

    pub async fn execute(
        &self,
        request: TranslationRunRequest,
    ) -> Result<(Job, Uuid), ApplicationError> {
        let admitted = self.managed.admit(request.clone()).await?;
        let job = admitted.job().clone();
        let job_id = job.id().clone();
        let run_id = admitted.run_id();
        if let Err(error) = self
            .runtime
            .reserve(job_id.clone(), request.project_id.clone())
            .await
        {
            return Err(self.compensate(run_id, &job_id, error.to_string()).await);
        }
        let (task, release) = managed_worker::spawn(
            admitted,
            request,
            self.runtime.clone(),
            self.jobs.clone(),
            self.pause.clone(),
        );
        if let Err(error) = self.runtime.attach_task(job_id.clone(), task).await {
            error.task.cancel.cancel();
            error.task.join_handle.abort();
            let _ = error.task.join_handle.await;
            self.runtime.finish_now(&job_id);
            return Err(self
                .compensate(run_id, &job_id, error.source.to_string())
                .await);
        }
        if release.send(()).is_err() {
            let failure = "translation worker stopped before activation".to_string();
            if let Err(error) = self.runtime.rollback_runtime_start(&job_id).await {
                tracing::error!(job_id = %job_id, error = %error, "runtime rollback needs recovery");
            }
            return Err(self.compensate(run_id, &job_id, failure).await);
        }
        Ok((job, run_id))
    }

    async fn compensate(
        &self,
        run_id: Uuid,
        job_id: &domain::job::JobId,
        scheduling_error: String,
    ) -> ApplicationError {
        match pending_job_failure::close_pending(self.jobs.as_ref(), run_id, job_id).await {
            Ok(()) => ApplicationError::PipelineStartFailed { scheduling_error },
            Err(error) => ApplicationError::PipelineStartFailedNeedsRecovery {
                scheduling_error,
                compensation_error: error.to_string(),
            },
        }
    }
}
