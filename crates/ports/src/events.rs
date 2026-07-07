use async_trait::async_trait;

use crate::error::PortError;
use domain::job::JobEvent;

#[async_trait]
pub trait JobEventPublisher: Send + Sync {
    async fn publish(&self, event: JobEvent) -> Result<(), PortError>;
}

#[async_trait]
pub trait AppEventPublisher: Send + Sync {
    async fn publish_project_updated(&self, project_id: &str) -> Result<(), PortError>;

    async fn publish_transcript_ready(
        &self,
        project_id: &str,
        job_id: &str,
    ) -> Result<(), PortError>;
}
