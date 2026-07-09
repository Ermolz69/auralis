use async_trait::async_trait;
use domain::media::{Artifact, ArtifactId, ArtifactKind};
use domain::project::ProjectId;

use crate::error::PortError;

#[async_trait]
pub trait ArtifactIndex: Send + Sync {
    async fn add(&self, project_id: &ProjectId, artifact: &Artifact) -> Result<(), PortError>;

    async fn get(&self, id: &ArtifactId) -> Result<Option<Artifact>, PortError>;

    async fn list_by_project(&self, project_id: &ProjectId) -> Result<Vec<Artifact>, PortError>;

    async fn list_by_project_and_kind(
        &self,
        project_id: &ProjectId,
        kind: ArtifactKind,
    ) -> Result<Vec<Artifact>, PortError>;

    async fn delete(&self, id: &ArtifactId) -> Result<(), PortError>;
}

use std::sync::Arc;

#[async_trait]
impl<T> ArtifactIndex for Arc<T>
where
    T: ArtifactIndex + ?Sized,
{
    async fn add(&self, project_id: &ProjectId, artifact: &Artifact) -> Result<(), PortError> {
        (**self).add(project_id, artifact).await
    }

    async fn get(&self, id: &ArtifactId) -> Result<Option<Artifact>, PortError> {
        (**self).get(id).await
    }

    async fn list_by_project(&self, project_id: &ProjectId) -> Result<Vec<Artifact>, PortError> {
        (**self).list_by_project(project_id).await
    }

    async fn list_by_project_and_kind(
        &self,
        project_id: &ProjectId,
        kind: ArtifactKind,
    ) -> Result<Vec<Artifact>, PortError> {
        (**self).list_by_project_and_kind(project_id, kind).await
    }

    async fn delete(&self, id: &ArtifactId) -> Result<(), PortError> {
        (**self).delete(id).await
    }
}
