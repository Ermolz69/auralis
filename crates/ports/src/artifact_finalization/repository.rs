use async_trait::async_trait;
use domain::{media::ArtifactId, project::ProjectId};

use super::FinalizationMetadata;
use crate::error::PortError;

#[async_trait]
pub trait ArtifactFinalizationLookup: Send + Sync {
    async fn get_for_finalization(
        &self,
        project_id: &ProjectId,
        artifact_id: &ArtifactId,
    ) -> Result<FinalizationMetadata, PortError>;
}

#[async_trait]
impl<T: ArtifactFinalizationLookup + ?Sized> ArtifactFinalizationLookup for std::sync::Arc<T> {
    async fn get_for_finalization(
        &self,
        project_id: &ProjectId,
        artifact_id: &ArtifactId,
    ) -> Result<FinalizationMetadata, PortError> {
        (**self).get_for_finalization(project_id, artifact_id).await
    }
}
