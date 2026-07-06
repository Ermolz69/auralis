use async_trait::async_trait;

use crate::error::PortError;
use domain::job::JobId;

#[async_trait]
pub trait JobControlPort: Send + Sync {
    async fn is_cancelled(&self, job_id: &JobId) -> Result<bool, PortError>;
}
