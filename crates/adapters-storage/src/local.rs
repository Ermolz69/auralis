use async_trait::async_trait;
use std::path::PathBuf;

use domain::media::{Artifact, ArtifactKind};
use domain::project::ProjectId;
use ports::error::PortError;
use ports::storage::ArtifactStore;

pub struct LocalArtifactStore {
    base_dir: PathBuf,
}

impl LocalArtifactStore {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }
}

#[async_trait]
impl ArtifactStore for LocalArtifactStore {
    async fn project_dir(&self, project_id: &ProjectId) -> Result<PathBuf, PortError> {
        let path = self.base_dir.join(project_id.to_string());
        Ok(path)
    }

    async fn reserve_artifact_path(
        &self,
        project_id: &ProjectId,
        kind: ArtifactKind,
        extension: &str,
    ) -> Result<PathBuf, PortError> {
        let dir = self.project_dir(project_id).await?;
        let filename = format!(
            "{:?}_{}.{}",
            kind,
            chrono::Utc::now().timestamp_millis(),
            extension
        );
        Ok(dir.join(filename))
    }

    async fn register_artifact(
        &self,
        _project_id: &ProjectId,
        _artifact: &Artifact,
    ) -> Result<(), PortError> {
        Ok(())
    }

    async fn resolve_artifact(&self, artifact: &Artifact) -> Result<PathBuf, PortError> {
        match artifact {
            Artifact {
                location: domain::media::ArtifactLocation::LocalPath(path),
                ..
            } => Ok(std::path::PathBuf::from(path)),
            _ => Err(PortError::Unexpected {
                message: "Artifact is not a local path".to_string(),
            }),
        }
    }

    async fn write_small_artifact(
        &self,
        project_id: &ProjectId,
        kind: ArtifactKind,
        filename: &str,
        _data: &[u8],
    ) -> Result<Artifact, PortError> {
        let dir = self.project_dir(project_id).await?;
        let path = dir.join(filename);
        Ok(Artifact {
            id: domain::media::ArtifactId(uuid::Uuid::new_v4()),
            kind,
            location: domain::media::ArtifactLocation::LocalPath(
                path.to_string_lossy().to_string(),
            ),
        })
    }
}
