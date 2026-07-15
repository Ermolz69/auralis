use domain::media::{Artifact, ArtifactId, ArtifactKind};
use domain::project::ProjectId;
use ports::error::PortError;
use std::path::Path;

use super::path_guard::ensure_safe_parent;
use super::resolver::resolve_storage_key;
use crate::local::storage_key::make_storage_key;

#[async_trait::async_trait]
pub(crate) trait FileOps: Send + Sync {
    async fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()>;
    async fn copy(&self, from: &Path, to: &Path) -> std::io::Result<u64>;
    async fn remove_file(&self, path: &Path) -> std::io::Result<()>;
}

pub(crate) struct DefaultFileOps;

#[async_trait::async_trait]
impl FileOps for DefaultFileOps {
    async fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        tokio::fs::rename(from, to).await
    }
    async fn copy(&self, from: &Path, to: &Path) -> std::io::Result<u64> {
        tokio::fs::copy(from, to).await
    }
    async fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        tokio::fs::remove_file(path).await
    }
}

pub async fn stage_owned_temp_file(
    base_dir: &Path,
    project_id: &ProjectId,
    kind: ArtifactKind,
    source_path: &Path,
    filename_hint: Option<&str>,
) -> Result<ports::storage::StagedArtifact, PortError> {
    stage_owned_temp_file_with_ops(
        base_dir,
        project_id,
        kind,
        source_path,
        filename_hint,
        &DefaultFileOps,
    )
    .await
}

pub(crate) async fn stage_owned_temp_file_with_ops<F: FileOps>(
    base_dir: &Path,
    project_id: &ProjectId,
    kind: ArtifactKind,
    source_path: &Path,
    filename_hint: Option<&str>,
    ops: &F,
) -> Result<ports::storage::StagedArtifact, PortError> {
    let source_exists = tokio::fs::try_exists(source_path)
        .await
        .map_err(|e| PortError::Io {
            message: format!(
                "Failed to check if source path {:?} exists: {}",
                source_path, e
            ),
        })?;
    if !source_exists {
        return Err(PortError::Io {
            message: format!("Source path {:?} does not exist", source_path),
        });
    }

    let ext = if let Some(hint) = filename_hint {
        Path::new(hint)
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

    let staging_path = resolve_storage_key(base_dir, &staging_key)?;

    ensure_safe_parent(base_dir, &staging_path).await?;

    // Try rename first
    if let Err(e) = ops.rename(source_path, &staging_path).await {
        // Rename failed, try copy + remove
        if let Err(copy_err) = ops.copy(source_path, &staging_path).await {
            // Copy failed, make sure we don't leave a partial staging file
            let staging_rm_err = ops.remove_file(&staging_path).await;

            // IMPORTANT: Since this is an owned file, we must delete it even on failure
            let _source_rm_err = ops.remove_file(source_path).await;

            if staging_rm_err
                .as_ref()
                .is_err_and(|e| e.kind() != std::io::ErrorKind::NotFound)
            {
                let rollback_err = staging_rm_err.unwrap_err();
                return Err(PortError::Io {
                    message: format!(
                        "Failed to copy to {:?}: rename err: {}, copy err: {}. Rollback of staging copy also failed: {}",
                        staging_path, e, copy_err, rollback_err
                    ),
                });
            }

            return Err(PortError::Io {
                message: format!(
                    "Failed to copy to {:?}: rename err: {}, copy err: {}",
                    staging_path, e, copy_err
                ),
            });
        }

        // Remove source, and if it fails, we MUST rollback staging
        if let Err(rm_err) = ops.remove_file(source_path).await {
            if let Err(rollback_err) = ops.remove_file(&staging_path).await {
                return Err(PortError::Io {
                    message: format!(
                        "Failed to delete source {:?} after copy: {}. Rollback of staging copy also failed: {}",
                        source_path, rm_err, rollback_err
                    ),
                });
            }
            return Err(PortError::Io {
                message: format!(
                    "Failed to delete source {:?} after copy: {}. Staging copy rolled back.",
                    source_path, rm_err
                ),
            });
        }
    }

    let metadata = match tokio::fs::metadata(&staging_path).await {
        Ok(m) => m,
        Err(e) => {
            let _ = ops.remove_file(&staging_path).await;
            return Err(PortError::Io {
                message: format!("Failed to read metadata of {:?}: {}", staging_path, e),
            });
        }
    };

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

pub async fn import_external_file(
    base_dir: &Path,
    project_id: &ProjectId,
    kind: ArtifactKind,
    source_path: &Path,
    filename_hint: Option<&str>,
) -> Result<ports::storage::StagedArtifact, PortError> {
    let source_exists = tokio::fs::try_exists(source_path)
        .await
        .map_err(|e| PortError::Io {
            message: format!(
                "Failed to check if source path {:?} exists: {}",
                source_path, e
            ),
        })?;
    if !source_exists {
        return Err(PortError::Io {
            message: format!("Source path {:?} does not exist", source_path),
        });
    }

    let ext = if let Some(hint) = filename_hint {
        Path::new(hint)
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

    let staging_path = resolve_storage_key(base_dir, &staging_key)?;

    ensure_safe_parent(base_dir, &staging_path).await?;

    // Only copy, do not move or remove original
    if let Err(copy_err) = tokio::fs::copy(source_path, &staging_path).await {
        let _ = tokio::fs::remove_file(&staging_path).await;
        return Err(PortError::Io {
            message: format!(
                "Failed to copy from {:?} to {:?}: {}",
                source_path, staging_path, copy_err
            ),
        });
    }

    let metadata = match tokio::fs::metadata(&staging_path).await {
        Ok(m) => m,
        Err(e) => {
            let _ = tokio::fs::remove_file(&staging_path).await;
            return Err(PortError::Io {
                message: format!("Failed to read metadata of {:?}: {}", staging_path, e),
            });
        }
    };

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

pub async fn finalize_staged_artifact(
    base_dir: &Path,
    staging_key: &str,
    final_key: &str,
) -> Result<(), PortError> {
    let staging_path = resolve_storage_key(base_dir, staging_key)?;
    let final_path = resolve_storage_key(base_dir, final_key)?;

    let final_exists = tokio::fs::try_exists(&final_path)
        .await
        .map_err(|e| PortError::Io {
            message: format!(
                "Failed to check if final path {:?} exists: {}",
                final_path, e
            ),
        })?;
    if final_exists {
        return Ok(());
    }

    let staging_exists = tokio::fs::try_exists(&staging_path)
        .await
        .map_err(|e| PortError::Io {
            message: format!(
                "Failed to check if staging path {:?} exists: {}",
                staging_path, e
            ),
        })?;
    if !staging_exists {
        return Err(PortError::Io {
            message: format!(
                "Cannot finalize: both staging_key {} and final_key {} are missing",
                staging_key, final_key
            ),
        });
    }

    ensure_safe_parent(base_dir, &final_path).await?;

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
