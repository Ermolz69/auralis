use async_trait::async_trait;
use domain::project::ProjectId;
use std::sync::Arc;

use crate::error::PortError;

#[async_trait]
pub trait ProjectWorkspacePort: Send + Sync {
    async fn ensure_and_open(&self, project_id: &ProjectId) -> Result<(), PortError>;
}

#[async_trait]
impl<T> ProjectWorkspacePort for Arc<T>
where
    T: ProjectWorkspacePort + ?Sized,
{
    async fn ensure_and_open(&self, project_id: &ProjectId) -> Result<(), PortError> {
        (**self).ensure_and_open(project_id).await
    }
}
