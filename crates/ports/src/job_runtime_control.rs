use async_trait::async_trait;
use domain::job::JobId;
use std::collections::HashMap;

use crate::error::PortError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeState {
    Starting,
    Running,
    Cancelling,
    Finished,
}

#[derive(Debug, Clone)]
pub enum JobCleanupStatus {
    NotRegistered,
    RemovedFromQueue,
    CooperativelyStopped,
    Aborted,
    AlreadyFinished,
    AbortUnconfirmed,
    CleanupFailed(String),
}

#[derive(Debug, Clone)]
pub struct RuntimeCleanupReport {
    pub jobs: HashMap<JobId, JobCleanupStatus>,
}

#[async_trait::async_trait]
pub trait JobRuntimeControlPort: Send + Sync {
    /// Cancels jobs, waits for tasks to finish, and safely evicts them from cache and locks.
    async fn cancel_and_evict_jobs(
        &self,
        job_ids: &[JobId],
    ) -> Result<RuntimeCleanupReport, PortError>;

    /// Registers a spawned task with the runtime control so it can be cancelled later.
    async fn register_runtime_task(
        &self,
        job_id: JobId,
        cancel_handle: crate::cancellation::CancelHandle,
        state_rx: tokio::sync::watch::Receiver<RuntimeState>,
        abort_handle: tokio::task::AbortHandle,
    );
}
