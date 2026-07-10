use domain::media::{Artifact, ArtifactId, ArtifactKind};
use domain::project::ProjectId;
use ports::error::PortError;
use std::path::Path;

use super::path_guard::ensure_safe_parent;
use super::resolver::resolve_storage_key;
use crate::local::storage_key::make_storage_key;

pub async fn stage_owned_temp_file(
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
    tokio::fs::copy(source_path, &staging_path)
        .await
        .map_err(|copy_err| PortError::Io {
            message: format!(
                "Failed to copy from {:?} to {:?}: {}",
                source_path, staging_path, copy_err
            ),
        })?;

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
