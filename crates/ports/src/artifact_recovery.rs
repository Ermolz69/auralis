use crate::error::PortError;
use async_trait::async_trait;
use domain::project::ProjectId;

#[async_trait]
pub trait ArtifactRecoveryRepository: Send + Sync {
    async fn list_projects(&self) -> Result<Vec<ProjectId>, PortError>;
    async fn retry_finalization(&self, project_id: &ProjectId) -> Result<(), PortError>;
}
