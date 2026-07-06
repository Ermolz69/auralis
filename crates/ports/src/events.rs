use async_trait::async_trait;

use crate::error::PortError;
use domain::job::JobEvent;

#[async_trait]
pub trait JobEventPublisher: Send + Sync {
    async fn publish(&self, event: JobEvent) -> Result<(), PortError>;
}
