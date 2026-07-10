use async_trait::async_trait;
use domain::media::{Artifact, ArtifactKind};
use domain::project::ProjectId;
use ports::error::PortError;
use ports::storage::{ArtifactStore, StagedArtifact};
use std::path::Path;

#[derive(Clone, Default)]
pub struct MockArtifactStore;

#[async_trait]
impl ArtifactStore for MockArtifactStore {
    async fn write_small_artifact(
        &self,
        _project_id: &ProjectId,
        _kind: ArtifactKind,
        _filename: &str,
        _data: &[u8],
    ) -> Result<Artifact, PortError> {
        unimplemented!()
    }

    async fn stage_owned_temp_file(
        &self,
        _project_id: &ProjectId,
        _kind: ArtifactKind,
        _temp_path: &Path,
        _filename_hint: Option<&str>,
    ) -> Result<StagedArtifact, PortError> {
        unimplemented!()
    }

    async fn import_external_file(
        &self,
        _project_id: &ProjectId,
        _kind: ArtifactKind,
        _source_path: &Path,
        _filename_hint: Option<&str>,
    ) -> Result<StagedArtifact, PortError> {
        unimplemented!()
    }

    async fn cleanup_stale_staging(&self, _max_age: std::time::Duration) -> Result<(), PortError> {
        unimplemented!()
    }

    async fn finalize_staged_artifact(
        &self,
        _staging_key: &str,
        _final_key: &str,
    ) -> Result<(), PortError> {
        unimplemented!()
    }

    async fn resolve_artifact(
        &self,
        _artifact: &Artifact,
    ) -> Result<std::path::PathBuf, PortError> {
        unimplemented!()
    }

    async fn delete_storage_key(&self, _key: &str) -> Result<(), PortError> {
        Ok(())
    }

    async fn delete_artifact(&self, _artifact: &Artifact) -> Result<(), PortError> {
        Ok(())
    }

    async fn delete_project_dir(&self, _project_id: &ProjectId) -> Result<(), PortError> {
        Ok(())
    }
}
