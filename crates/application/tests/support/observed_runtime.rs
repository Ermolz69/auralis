use async_trait::async_trait;
use domain::{job::JobId, project::ProjectId};
use ports::{
    PortError,
    job_runtime_control::{
        AttachTaskError, JobRuntimeControlPort, RuntimeCleanupOutcome, RuntimeCleanupReport,
        RuntimeCompletion, RuntimeShutdownReport, RuntimeTask,
    },
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

pub struct ObservedRuntime {
    inner: Arc<dyn JobRuntimeControlPort>,
    completion: Mutex<Option<Arc<RuntimeCompletion>>>,
}

impl ObservedRuntime {
    pub fn new(inner: Arc<dyn JobRuntimeControlPort>) -> Self {
        Self {
            inner,
            completion: Mutex::new(None),
        }
    }
    pub fn completion(&self) -> Result<Arc<RuntimeCompletion>, Box<dyn std::error::Error>> {
        self.completion
            .lock()
            .map_err(|_| "Runtime observer lock poisoned")?
            .clone()
            .ok_or_else(|| "Task was not attached".into())
    }
}

#[async_trait]
impl JobRuntimeControlPort for ObservedRuntime {
    async fn reserve(&self, id: JobId, project: ProjectId) -> Result<(), PortError> {
        self.inner.reserve(id, project).await
    }
    async fn attach_task(&self, id: JobId, task: RuntimeTask) -> Result<(), AttachTaskError> {
        let completion = task.completion.clone();
        self.inner.attach_task(id, task).await?;
        *self
            .completion
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(completion);
        Ok(())
    }
    fn finish_now(&self, id: &JobId) {
        self.inner.finish_now(id);
    }
    async fn rollback_runtime_start(&self, id: &JobId) -> Result<RuntimeCleanupOutcome, PortError> {
        self.inner.rollback_runtime_start(id).await
    }
    async fn cancel_and_evict_jobs(
        &self,
        ids: &[JobId],
    ) -> Result<RuntimeCleanupReport, PortError> {
        self.inner.cancel_and_evict_jobs(ids).await
    }
    async fn drain_all(&self, deadline: Duration) -> Result<RuntimeShutdownReport, PortError> {
        self.inner.drain_all(deadline).await
    }
}
