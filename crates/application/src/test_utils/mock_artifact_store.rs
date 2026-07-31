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
    async fn stage_owned_temp_file(
        &self,
        project_id: &ProjectId,
        kind: ArtifactKind,
        _temp_path: &Path,
        filename_hint: Option<&str>,
    ) -> Result<StagedArtifact, PortError> {
        let artifact = Artifact {
            id: domain::media::ArtifactId::new(),
            kind: kind.clone(),
            location: domain::media::ArtifactLocation::LocalPath(
                filename_hint.unwrap_or("test.ext").to_string(),
            ),
            size_bytes: Some(1024),
            state: domain::media::ArtifactState::PendingFinalize,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            ready_at: None,
        };
        Ok(StagedArtifact {
            artifact,
            staging_key: format!("staged_{}_{:?}", project_id, kind),
            final_key: format!("final_{}_{:?}", project_id, kind),
            size_bytes: 1024,
        })
    }

    async fn import_external_file(
        &self,
        project_id: &ProjectId,
        kind: ArtifactKind,
        _source_path: &Path,
        filename_hint: Option<&str>,
    ) -> Result<StagedArtifact, PortError> {
        let artifact = Artifact {
            id: domain::media::ArtifactId::new(),
            kind: kind.clone(),
            location: domain::media::ArtifactLocation::LocalPath(
                filename_hint.unwrap_or("test.ext").to_string(),
            ),
            size_bytes: Some(1024),
            state: domain::media::ArtifactState::PendingFinalize,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            ready_at: None,
        };
        Ok(StagedArtifact {
            artifact,
            staging_key: format!("staged_{}_{:?}", project_id, kind),
            final_key: format!("final_{}_{:?}", project_id, kind),
            size_bytes: 1024,
        })
    }

    async fn cleanup_stale_staging(&self, _max_age: std::time::Duration) -> Result<(), PortError> {
        unimplemented!()
    }

    async fn finalize_staged_artifact(
        &self,
        _staging_key: &str,
        _final_key: &str,
    ) -> Result<(), PortError> {
        Ok(())
    }

    async fn resolve_artifact(
        &self,
        _artifact: &Artifact,
    ) -> Result<std::path::PathBuf, PortError> {
        Ok(std::path::PathBuf::from("/mock/path"))
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

    async fn stage_owned_workspace_file(
        &self,
        project_id: &ProjectId,
        kind: ArtifactKind,
        _workspace_port: &dyn ports::workspace::TempWorkspacePort,
        _allocation_key: &domain::outbox::WorkspaceKey,
        _relative_file: &str,
        filename_hint: Option<&str>,
    ) -> Result<StagedArtifact, PortError> {
        let dummy_path = Path::new("/mock/path");
        self.stage_owned_temp_file(project_id, kind, dummy_path, filename_hint)
            .await
    }
}
