use async_trait::async_trait;
use std::path::{Path, PathBuf};

use domain::media::{Artifact, ArtifactKind};
use domain::project::ProjectId;
use ports::error::PortError;
use ports::storage::ArtifactStore;

use super::cleanup;
use super::deletion;
use super::resolver;
use super::staging;

pub struct LocalArtifactStore {
    base_dir: PathBuf,
}

impl LocalArtifactStore {
    pub fn new(base_dir: PathBuf) -> Self {
        let _ = std::fs::create_dir_all(&base_dir);
        Self { base_dir }
    }

    pub fn resolve_storage_key(&self, key: &str) -> Result<PathBuf, PortError> {
        resolver::resolve_storage_key(&self.base_dir, key)
    }

    pub fn resolve_legacy_local_path(&self, path: &str) -> Result<PathBuf, PortError> {
        resolver::resolve_legacy_local_path(path)
    }
}

#[async_trait]
impl ArtifactStore for LocalArtifactStore {
    async fn resolve_artifact(&self, artifact: &Artifact) -> Result<PathBuf, PortError> {
        resolver::resolve_artifact(&self.base_dir, artifact)
    }

    async fn stage_owned_temp_file(
        &self,
        project_id: &ProjectId,
        kind: ArtifactKind,
        source_path: &Path,
        filename_hint: Option<&str>,
    ) -> Result<ports::storage::StagedArtifact, PortError> {
        staging::stage_owned_temp_file(&self.base_dir, project_id, kind, source_path, filename_hint)
            .await
    }

    async fn import_external_file(
        &self,
        project_id: &ProjectId,
        kind: ArtifactKind,
        source_path: &Path,
        filename_hint: Option<&str>,
    ) -> Result<ports::storage::StagedArtifact, PortError> {
        staging::import_external_file(&self.base_dir, project_id, kind, source_path, filename_hint)
            .await
    }

    async fn finalize_staged_artifact(
        &self,
        staging_key: &str,
        final_key: &str,
    ) -> Result<(), PortError> {
        staging::finalize_staged_artifact(&self.base_dir, staging_key, final_key).await
    }

    async fn delete_storage_key(&self, storage_key: &str) -> Result<(), PortError> {
        deletion::delete_storage_key(&self.base_dir, storage_key).await
    }

    async fn delete_artifact(&self, artifact: &Artifact) -> Result<(), PortError> {
        deletion::delete_artifact(&self.base_dir, artifact).await
    }

    async fn delete_project_dir(&self, project_id: &ProjectId) -> Result<(), PortError> {
        deletion::delete_project_dir(&self.base_dir, project_id).await
    }

    async fn cleanup_stale_staging(&self, max_age: std::time::Duration) -> Result<(), PortError> {
        cleanup::cleanup_stale_staging(&self.base_dir, max_age).await
    }
}
