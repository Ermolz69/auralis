use async_trait::async_trait;

use domain::job::JobId;
use crate::error::PortError;

#[async_trait]
pub trait JobControlPort: Send + Sync {
    async fn is_cancelled(&self, job_id: &JobId) -> Result<bool, PortError>;
}
