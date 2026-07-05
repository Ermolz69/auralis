use async_trait::async_trait;
use std::path::PathBuf;

use domain::media::{Artifact, ArtifactKind};
use domain::project::ProjectId;
use crate::error::PortError;

#[async_trait]
pub trait ArtifactStore: Send + Sync {
    async fn project_dir(&self, project_id: &ProjectId) -> Result<PathBuf, PortError>;

    async fn reserve_artifact_path(
        &self,
        project_id: &ProjectId,
        kind: ArtifactKind,
        extension: &str,
    ) -> Result<PathBuf, PortError>;

    async fn register_artifact(
        &self,
        project_id: &ProjectId,
        artifact: &Artifact,
    ) -> Result<(), PortError>;

    async fn resolve_artifact(
        &self,
        artifact: &Artifact,
    ) -> Result<PathBuf, PortError>;

    async fn write_small_artifact(
        &self,
        project_id: &ProjectId,
        kind: ArtifactKind,
        filename: &str,
        data: &[u8],
    ) -> Result<Artifact, PortError>;
}
