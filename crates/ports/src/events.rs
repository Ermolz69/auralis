use async_trait::async_trait;

use domain::job::JobEvent;
use crate::error::PortError;

#[async_trait]
pub trait JobEventPublisher: Send + Sync {
    async fn publish(&self, event: JobEvent) -> Result<(), PortError>;
}
