use async_trait::async_trait;
use std::path::PathBuf;

use domain::media::{Artifact, ArtifactId, ArtifactKind};
use domain::project::ProjectId;
use ports::error::PortError;
use ports::storage::ArtifactStore;

use super::storage_key::make_storage_key;

pub struct LocalArtifactStore {
    base_dir: PathBuf,
}

impl LocalArtifactStore {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn resolve_storage_key(&self, key: &str) -> PathBuf {
        self.base_dir.join(key)
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
        let artifact_id = ArtifactId(uuid::Uuid::new_v4());
        let storage_key = make_storage_key(project_id, &artifact_id, &kind, extension);
        Ok(self.resolve_storage_key(&storage_key))
    }

    async fn register_artifact(
        &self,
        _project_id: &ProjectId,
        _artifact: &Artifact,
    ) -> Result<(), PortError> {
        Ok(())
    }

    async fn resolve_artifact(&self, artifact: &Artifact) -> Result<PathBuf, PortError> {
        match &artifact.location {
            domain::media::ArtifactLocation::LocalPath(path) => Ok(std::path::PathBuf::from(path)),
            domain::media::ArtifactLocation::StorageKey(key) => Ok(self.resolve_storage_key(key)),
        }
    }

    async fn write_small_artifact(
        &self,
        project_id: &ProjectId,
        kind: ArtifactKind,
        filename: &str,
        data: &[u8],
    ) -> Result<Artifact, PortError> {
        let artifact_id = ArtifactId(uuid::Uuid::new_v4());
        let ext = std::path::Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin");

        let storage_key = make_storage_key(project_id, &artifact_id, &kind, ext);
        let path = self.resolve_storage_key(&storage_key);

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| PortError::Io {
                    message: format!("Failed to create directory {:?}: {}", parent, e),
                })?;
        }

        tokio::fs::write(&path, data)
            .await
            .map_err(|e| PortError::Io {
                message: format!("Failed to write to {:?}: {}", path, e),
            })?;

        Ok(Artifact {
            id: artifact_id,
            kind,
            location: domain::media::ArtifactLocation::StorageKey(storage_key),
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
            domain::media::ArtifactLocation::StorageKey(key) => {
                let path = store.resolve_storage_key(&key);
                let saved_data = tokio::fs::read(&path).await.expect("File should exist");
                assert_eq!(saved_data, b"hello world");
            }
            _ => panic!("Expected StorageKey"),
        }
    }

    #[tokio::test]
    async fn test_resolve_artifact() {
        let temp_dir = tempdir().unwrap();
        let store = LocalArtifactStore::new(temp_dir.path().to_path_buf());
        let project_id = ProjectId(uuid::Uuid::new_v4());

        let artifact = store
            .write_small_artifact(
                &project_id,
                ArtifactKind::LogFile,
                "test.log",
                b"test",
            )
            .await
            .unwrap();

        let resolved_path = store.resolve_artifact(&artifact).await.unwrap();
        assert!(resolved_path.starts_with(temp_dir.path()));
        
        // Also check legacy LocalPath
        let legacy_artifact = Artifact {
            id: domain::media::ArtifactId(uuid::Uuid::new_v4()),
            kind: ArtifactKind::LogFile,
            location: domain::media::ArtifactLocation::LocalPath("/tmp/legacy.log".to_string()),
        };
        let legacy_path = store.resolve_artifact(&legacy_artifact).await.unwrap();
        assert_eq!(legacy_path, std::path::PathBuf::from("/tmp/legacy.log"));
    }
}
