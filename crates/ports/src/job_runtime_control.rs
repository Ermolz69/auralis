use async_trait::async_trait;
use domain::job::JobId;

use crate::error::PortError;

#[async_trait]
pub trait JobRuntimeControlPort: Send + Sync {
    /// Cancels jobs, waits for tasks to finish, and safely evicts them from cache and locks.
    async fn cancel_and_evict_jobs(&self, job_ids: &[JobId]) -> Result<(), PortError>;
}
