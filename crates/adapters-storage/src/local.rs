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
        data: &[u8],
    ) -> Result<Artifact, PortError> {
        let dir = self.project_dir(project_id).await?;
        let path = dir.join(filename);

        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| PortError::Io {
                message: format!("Failed to create directory {:?}: {}", dir, e),
            })?;

        tokio::fs::write(&path, data)
            .await
            .map_err(|e| PortError::Io {
                message: format!("Failed to write to {:?}: {}", path, e),
            })?;

        Ok(Artifact {
            id: domain::media::ArtifactId(uuid::Uuid::new_v4()),
            kind,
            location: domain::media::ArtifactLocation::LocalPath(
                path.to_string_lossy().to_string(),
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_write_small_artifact() {
        let temp_dir = tempdir().unwrap();
        let store = LocalArtifactStore::new(temp_dir.path().to_path_buf());
        let project_id = ProjectId(uuid::Uuid::new_v4());

        let result = store
            .write_small_artifact(
                &project_id,
                ArtifactKind::LogFile,
                "test_file.txt",
                b"hello world",
            )
            .await;

        assert!(result.is_ok());
        let artifact = result.unwrap();

        match artifact.location {
            domain::media::ArtifactLocation::LocalPath(path_str) => {
                let saved_data = tokio::fs::read(&path_str).await.expect("File should exist");
                assert_eq!(saved_data, b"hello world");
            }
            _ => panic!("Expected LocalPath"),
        }
    }
}
