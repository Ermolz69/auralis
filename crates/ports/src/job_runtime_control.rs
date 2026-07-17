use domain::job::JobId;
use domain::project::ProjectId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinHandle;

use crate::error::PortError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeTaskOutcome {
    Completed,
    Cancelled,
    ApplicationFailed,
    Panicked,
}

pub struct RuntimeCompletion {
    pub state: std::sync::atomic::AtomicU8,
    pub outcome: std::sync::Mutex<Option<RuntimeTaskOutcome>>,
    pub notify: tokio::sync::Notify,
}

impl Default for RuntimeCompletion {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeCompletion {
    pub fn new() -> Self {
        Self {
            state: std::sync::atomic::AtomicU8::new(0),
            outcome: std::sync::Mutex::new(None),
            notify: tokio::sync::Notify::new(),
        }
    }
}

pub struct RuntimeTask {
    pub cancel: crate::cancellation::CancelHandle,
    pub join_handle: JoinHandle<RuntimeTaskOutcome>,
    pub completion: Arc<RuntimeCompletion>,
}

impl std::fmt::Debug for RuntimeTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeTask").finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct AttachTaskError {
    pub source: PortError,
    pub task: RuntimeTask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeCleanupOutcome {
    Missing,
    ReservationRemoved,
    Completed,
    CooperativeCancelled,
    ForcedAborted,
    Panicked,
    JoinFailed,
    Unconfirmed,
}

#[derive(Debug, Clone)]
pub struct RuntimeCleanupReport {
    pub jobs: HashMap<JobId, RuntimeCleanupOutcome>,
}

#[async_trait::async_trait]
pub trait JobRuntimeControlPort: Send + Sync {
    async fn reserve(&self, job_id: JobId, project_id: ProjectId) -> Result<(), PortError>;

    async fn attach_task(&self, job_id: JobId, task: RuntimeTask) -> Result<(), AttachTaskError>;

    fn finish_now(&self, job_id: &JobId);

    async fn rollback_runtime_start(
        &self,
        job_id: &JobId,
    ) -> Result<RuntimeCleanupOutcome, PortError>;

    async fn cancel_and_evict_jobs(
        &self,
        job_ids: &[JobId],
    ) -> Result<RuntimeCleanupReport, PortError>;
}
