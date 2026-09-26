use domain::{job::JobId, project::ProjectId};
use ports::error::PortError;
use ports::job_runtime_control::{
    AttachTaskError, JobRuntimeControlPort, RuntimeCleanupOutcome, RuntimeCleanupReport,
    RuntimeCompletion, RuntimeTask,
};
use std::sync::{Arc, Mutex};

pub struct ObservedJobRuntime {
    inner: Arc<dyn JobRuntimeControlPort>,
    completion: Mutex<Option<Arc<RuntimeCompletion>>>,
}

impl ObservedJobRuntime {
    pub fn new(inner: Arc<dyn JobRuntimeControlPort>) -> Self {
        Self {
            inner,
            completion: Mutex::new(None),
        }
    }

    pub fn completion(&self) -> Option<Arc<RuntimeCompletion>> {
        self.completion
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

#[async_trait::async_trait]
impl JobRuntimeControlPort for ObservedJobRuntime {
    async fn reserve(&self, job: JobId, project: ProjectId) -> Result<(), PortError> {
        self.inner.reserve(job, project).await
    }

    async fn attach_task(&self, job: JobId, task: RuntimeTask) -> Result<(), AttachTaskError> {
        *self
            .completion
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(task.completion.clone());
        self.inner.attach_task(job, task).await
    }

    fn finish_now(&self, job: &JobId) {
        self.inner.finish_now(job);
    }

    async fn rollback_runtime_start(
        &self,
        job: &JobId,
    ) -> Result<RuntimeCleanupOutcome, PortError> {
        self.inner.rollback_runtime_start(job).await
    }

    async fn cancel_and_evict_jobs(
        &self,
        jobs: &[JobId],
    ) -> Result<RuntimeCleanupReport, PortError> {
        self.inner.cancel_and_evict_jobs(jobs).await
    }
}
