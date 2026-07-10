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

    pub fn resolve_storage_key(&self, key: &str) -> Result<PathBuf, PortError> {
        let key_path = PathBuf::from(key);

        if key_path.is_absolute() {
            return Err(PortError::Unexpected {
                message: "StorageKey must be relative".into(),
            });
        }

        if key_path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(PortError::Unexpected {
                message: "StorageKey must not contain parent directory traversal".into(),
            });
        }

        Ok(self.base_dir.join(key_path))
    }

    pub fn resolve_legacy_local_path(&self, path: &str) -> Result<PathBuf, PortError> {
        // legacy only: artifacts created by old builds
        Ok(PathBuf::from(path))
    }
}

#[async_trait]
impl ArtifactStore for LocalArtifactStore {
    async fn write_small_artifact(
        &self,
        project_id: &ProjectId,
        kind: ArtifactKind,
        filename: &str,
        data: &[u8],
    ) -> Result<Artifact, PortError> {
        let artifact_id = ArtifactId::new();
        let ext = std::path::Path::new(filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin");

        let storage_key = make_storage_key(project_id, &artifact_id, &kind, ext);
        let path = self.resolve_storage_key(&storage_key)?;

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
            size_bytes: Some(data.len() as u64),
            state: domain::media::ArtifactState::Ready,
            created_at: domain::chrono::Utc::now(),
            updated_at: domain::chrono::Utc::now(),
            ready_at: Some(domain::chrono::Utc::now()),
        })
    }

    async fn resolve_artifact(&self, artifact: &Artifact) -> Result<PathBuf, PortError> {
        match &artifact.location {
            domain::media::ArtifactLocation::LocalPath(_) => {
                Err(PortError::Unsupported {
                    message: "Legacy external artifacts cannot be resolved through general resolve_artifact. Use a migration service.".to_string(),
                })
            }
            domain::media::ArtifactLocation::StorageKey(key) => self.resolve_storage_key(key),
        }
    }
    async fn stage_external_file(
        &self,
        project_id: &ProjectId,
        kind: ArtifactKind,
        source_path: &std::path::Path,
        filename_hint: Option<&str>,
    ) -> Result<ports::storage::StagedArtifact, PortError> {
        if !tokio::fs::try_exists(source_path).await.unwrap_or(false) {
            return Err(PortError::Io {
                message: format!("Source path {:?} does not exist", source_path),
            });
        }

        let ext = if let Some(hint) = filename_hint {
            std::path::Path::new(hint)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("bin")
        } else {
            source_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("bin")
        };

        let artifact_id = ArtifactId::new();
        let final_key = make_storage_key(project_id, &artifact_id, &kind, ext);
        let staging_key = format!(".staging/{}/{}.{}", uuid::Uuid::new_v4(), artifact_id, ext);

        let staging_path = self.resolve_storage_key(&staging_key)?;

        if let Some(parent) = staging_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| PortError::Io {
                    message: format!("Failed to create directory {:?}: {}", parent, e),
                })?;
        }

        // Try rename first
        if let Err(e) = tokio::fs::rename(source_path, &staging_path).await {
            // Rename failed, try copy + remove
            tokio::fs::copy(source_path, &staging_path)
                .await
                .map_err(|copy_err| PortError::Io {
                    message: format!(
                        "Failed to copy to {:?}: rename err: {}, copy err: {}",
                        staging_path, e, copy_err
                    ),
                })?;

            // Remove source best-effort
            let _ = tokio::fs::remove_file(source_path).await;
        }

        let metadata = tokio::fs::metadata(&staging_path)
            .await
            .map_err(|e| PortError::Io {
                message: format!("Failed to read metadata of {:?}: {}", staging_path, e),
            })?;

        let size_bytes = metadata.len();

        let artifact = Artifact {
            id: artifact_id,
            kind,
            location: domain::media::ArtifactLocation::StorageKey(final_key.clone()),
            size_bytes: Some(size_bytes),
            state: domain::media::ArtifactState::PendingFinalize,
            created_at: domain::chrono::Utc::now(),
            updated_at: domain::chrono::Utc::now(),
            ready_at: None,
        };

        Ok(ports::storage::StagedArtifact {
            artifact,
            staging_key,
            final_key,
            size_bytes,
        })
    }

    async fn finalize_staged_artifact(
        &self,
        staging_key: &str,
        final_key: &str,
    ) -> Result<(), PortError> {
        let staging_path = self.resolve_storage_key(staging_key)?;
        let final_path = self.resolve_storage_key(final_key)?;

        let final_exists = tokio::fs::try_exists(&final_path).await.unwrap_or(false);
        if final_exists {
            return Ok(());
        }

        let staging_exists = tokio::fs::try_exists(&staging_path).await.unwrap_or(false);
        if !staging_exists {
            return Err(PortError::Io {
                message: format!(
                    "Cannot finalize: both staging_key {} and final_key {} are missing",
                    staging_key, final_key
                ),
            });
        }

        if let Some(parent) = final_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| PortError::Io {
                    message: format!("Failed to create directory {:?}: {}", parent, e),
                })?;
        }

        tokio::fs::rename(&staging_path, &final_path)
            .await
            .map_err(|e| PortError::Io {
                message: format!(
                    "Failed to finalize staging {} to {}: {}",
                    staging_key, final_key, e
                ),
            })?;

        Ok(())
    }

    async fn delete_storage_key(&self, storage_key: &str) -> Result<(), PortError> {
        let path = self.resolve_storage_key(storage_key)?;
        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            tokio::fs::remove_file(&path)
                .await
                .map_err(|e| PortError::Io {
                    message: format!("Failed to delete file {:?}: {}", path, e),
                })?;
        }
        Ok(())
    }

    async fn delete_artifact(&self, artifact: &Artifact) -> Result<(), PortError> {
        if let domain::media::ArtifactLocation::StorageKey(key) = &artifact.location {
            self.delete_storage_key(key).await?;
        }
        Ok(())
    }

    async fn delete_project_dir(&self, project_id: &ProjectId) -> Result<(), PortError> {
        // ... (we'll implement this below)
        let path = self.base_dir.join(project_id.to_string());
        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            tokio::fs::remove_dir_all(&path)
                .await
                .map_err(|e| PortError::Io {
                    message: format!("Failed to delete project directory {:?}: {}", path, e),
                })?;
        }
        Ok(())
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
                let path = store.resolve_storage_key(&key).unwrap();
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
            .write_small_artifact(&project_id, ArtifactKind::LogFile, "test.log", b"test")
            .await
            .unwrap();

        let resolved_path = store.resolve_artifact(&artifact).await.unwrap();
        assert!(resolved_path.starts_with(temp_dir.path()));

        // Also check legacy LocalPath
        let legacy_artifact = Artifact {
            id: domain::media::ArtifactId(uuid::Uuid::new_v4()),
            kind: ArtifactKind::LogFile,
            location: domain::media::ArtifactLocation::LocalPath("/tmp/legacy.log".to_string()),
            size_bytes: None,
            state: domain::media::ArtifactState::Ready,
            created_at: domain::chrono::Utc::now(),
            updated_at: domain::chrono::Utc::now(),
            ready_at: Some(domain::chrono::Utc::now()),
        };
        let legacy_path = store.resolve_artifact(&legacy_artifact).await.unwrap();
        assert_eq!(legacy_path, std::path::PathBuf::from("/tmp/legacy.log"));
    }

    #[tokio::test]
    async fn test_stage_external_file_creates_pending_artifact() {
        let temp_dir = tempdir().unwrap();
        let store = LocalArtifactStore::new(temp_dir.path().to_path_buf());
        let project_id = ProjectId(uuid::Uuid::new_v4());

        let source_dir = tempdir().unwrap();
        let source_path = source_dir.path().join("video.mp4");
        tokio::fs::write(&source_path, b"video data").await.unwrap();

        let staged = store
            .stage_external_file(&project_id, ArtifactKind::SourceVideo, &source_path, None)
            .await
            .unwrap();

        assert_eq!(
            staged.artifact.state,
            domain::media::ArtifactState::PendingFinalize
        );
        assert_eq!(staged.size_bytes, 10);
        assert!(staged.staging_key.starts_with(".staging/"));
        assert!(!tokio::fs::try_exists(&source_path).await.unwrap());
        let staging_path = store.resolve_storage_key(&staged.staging_key).unwrap();
        assert!(tokio::fs::try_exists(&staging_path).await.unwrap());
    }

    #[tokio::test]
    async fn test_finalize_moves_to_final() {
        let temp_dir = tempdir().unwrap();
        let store = LocalArtifactStore::new(temp_dir.path().to_path_buf());
        let project_id = ProjectId(uuid::Uuid::new_v4());

        let source_dir = tempdir().unwrap();
        let source_path = source_dir.path().join("video.mp4");
        tokio::fs::write(&source_path, b"video data").await.unwrap();

        let staged = store
            .stage_external_file(&project_id, ArtifactKind::SourceVideo, &source_path, None)
            .await
            .unwrap();

        store
            .finalize_staged_artifact(&staged.staging_key, &staged.final_key)
            .await
            .unwrap();

        let staging_path = store.resolve_storage_key(&staged.staging_key).unwrap();
        let final_path = store.resolve_storage_key(&staged.final_key).unwrap();

        assert!(!tokio::fs::try_exists(&staging_path).await.unwrap());
        assert!(tokio::fs::try_exists(&final_path).await.unwrap());
    }

    #[tokio::test]
    async fn test_finalize_is_idempotent_when_final_exists() {
        let temp_dir = tempdir().unwrap();
        let store = LocalArtifactStore::new(temp_dir.path().to_path_buf());
        let project_id = ProjectId(uuid::Uuid::new_v4());

        let source_dir = tempdir().unwrap();
        let source_path = source_dir.path().join("video.mp4");
        tokio::fs::write(&source_path, b"video data").await.unwrap();

        let staged = store
            .stage_external_file(&project_id, ArtifactKind::SourceVideo, &source_path, None)
            .await
            .unwrap();

        store
            .finalize_staged_artifact(&staged.staging_key, &staged.final_key)
            .await
            .unwrap();

        // Finalize again should be ok
        let result = store
            .finalize_staged_artifact(&staged.staging_key, &staged.final_key)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_finalize_fails_when_both_missing() {
        let temp_dir = tempdir().unwrap();
        let store = LocalArtifactStore::new(temp_dir.path().to_path_buf());

        let result = store
            .finalize_staged_artifact(".staging/missing", "missing_final.txt")
            .await;
        assert!(result.is_err());
        if let Err(PortError::Io { message }) = result {
            assert!(message.contains("missing"));
        } else {
            panic!("Expected Io error");
        }
    }

    #[tokio::test]
    async fn test_delete_storage_key_is_idempotent() {
        let temp_dir = tempdir().unwrap();
        let store = LocalArtifactStore::new(temp_dir.path().to_path_buf());

        let result = store.delete_storage_key("some_missing_key.txt").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_storage_key_rejects_absolute_path() {
        let temp_dir = tempdir().unwrap();
        let store = LocalArtifactStore::new(temp_dir.path().to_path_buf());

        #[cfg(target_os = "windows")]
        let key = "C:\\Windows\\System32\\cmd.exe";
        #[cfg(not(target_os = "windows"))]
        let key = "/etc/passwd";

        let result = store.resolve_storage_key(key);
        assert!(result.is_err());
        if let Err(PortError::Unexpected { message }) = result {
            assert!(message.contains("must be relative"));
        } else {
            panic!("Expected Unexpected error");
        }
    }

    #[tokio::test]
    async fn test_storage_key_rejects_parent_dir() {
        let temp_dir = tempdir().unwrap();
        let store = LocalArtifactStore::new(temp_dir.path().to_path_buf());

        let result = store.resolve_storage_key("some/../../path.txt");
        assert!(result.is_err());
        if let Err(PortError::Unexpected { message }) = result {
            assert!(message.contains("parent directory traversal"));
        } else {
            panic!("Expected Unexpected error");
        }
    }
}
