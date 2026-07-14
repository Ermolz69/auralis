use async_trait::async_trait;
use std::path::PathBuf;

use crate::error::PortError;
use domain::media::{Artifact, ArtifactKind};
use domain::project::ProjectId;

pub struct StagedArtifact {
    pub artifact: Artifact,
    pub staging_key: String,
    pub final_key: String,
    pub size_bytes: u64,
}

#[async_trait]
pub trait ArtifactStore: Send + Sync {
    async fn stage_owned_temp_file(
        &self,
        project_id: &ProjectId,
        kind: ArtifactKind,
        source_path: &std::path::Path,
        filename_hint: Option<&str>,
    ) -> Result<StagedArtifact, PortError>;

    async fn import_external_file(
        &self,
        project_id: &ProjectId,
        kind: ArtifactKind,
        source_path: &std::path::Path,
        filename_hint: Option<&str>,
    ) -> Result<StagedArtifact, PortError>;

    async fn finalize_staged_artifact(
        &self,
        staging_key: &str,
        final_key: &str,
    ) -> Result<(), PortError>;

    async fn resolve_artifact(&self, artifact: &Artifact) -> Result<PathBuf, PortError>;

    async fn delete_storage_key(&self, storage_key: &str) -> Result<(), PortError>;

    async fn delete_artifact(&self, artifact: &Artifact) -> Result<(), PortError>;

    async fn delete_project_dir(&self, project_id: &ProjectId) -> Result<(), PortError>;
    async fn cleanup_stale_staging(&self, max_age: std::time::Duration) -> Result<(), PortError>;
}

use std::sync::Arc;

#[async_trait]
impl<T> ArtifactStore for Arc<T>
where
    T: ArtifactStore + ?Sized,
{
    async fn stage_owned_temp_file(
        &self,
        project_id: &ProjectId,
        kind: ArtifactKind,
        source_path: &std::path::Path,
        filename_hint: Option<&str>,
    ) -> Result<StagedArtifact, PortError> {
        (**self)
            .stage_owned_temp_file(project_id, kind, source_path, filename_hint)
            .await
    }

    async fn import_external_file(
        &self,
        project_id: &ProjectId,
        kind: ArtifactKind,
        source_path: &std::path::Path,
        filename_hint: Option<&str>,
    ) -> Result<StagedArtifact, PortError> {
        (**self)
            .import_external_file(project_id, kind, source_path, filename_hint)
            .await
    }

    async fn finalize_staged_artifact(
        &self,
        staging_key: &str,
        final_key: &str,
    ) -> Result<(), PortError> {
        (**self)
            .finalize_staged_artifact(staging_key, final_key)
            .await
    }

    async fn resolve_artifact(&self, artifact: &Artifact) -> Result<PathBuf, PortError> {
        (**self).resolve_artifact(artifact).await
    }

    async fn delete_storage_key(&self, storage_key: &str) -> Result<(), PortError> {
        (**self).delete_storage_key(storage_key).await
    }

    async fn delete_artifact(&self, artifact: &Artifact) -> Result<(), PortError> {
        (**self).delete_artifact(artifact).await
    }

    async fn cleanup_stale_staging(&self, max_age: std::time::Duration) -> Result<(), PortError> {
        (**self).cleanup_stale_staging(max_age).await
    }

    async fn delete_project_dir(&self, project_id: &ProjectId) -> Result<(), PortError> {
        (**self).delete_project_dir(project_id).await
    }
}
