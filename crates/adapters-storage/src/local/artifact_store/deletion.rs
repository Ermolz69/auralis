use domain::media::Artifact;
use domain::project::ProjectId;
use ports::error::PortError;
use std::path::Path;

use super::resolver::resolve_storage_key;

pub async fn delete_storage_key(base_dir: &Path, storage_key: &str) -> Result<(), PortError> {
    let path = resolve_storage_key(base_dir, storage_key)?;
    let exists = tokio::fs::try_exists(&path)
        .await
        .map_err(|e| PortError::Io {
            message: format!(
                "Failed to check if storage key path {:?} exists: {}",
                path, e
            ),
        })?;
    if exists {
        tokio::fs::remove_file(&path)
            .await
            .map_err(|e| PortError::Io {
                message: format!("Failed to delete file {:?}: {}", path, e),
            })?;
    }
    Ok(())
}

pub async fn delete_artifact(base_dir: &Path, artifact: &Artifact) -> Result<(), PortError> {
    if let domain::media::ArtifactLocation::StorageKey(key) = &artifact.location {
        delete_storage_key(base_dir, key).await?;
    }
    Ok(())
}

pub async fn delete_project_dir(base_dir: &Path, project_id: &ProjectId) -> Result<(), PortError> {
    let path = base_dir.join(project_id.to_string());
    let exists = tokio::fs::try_exists(&path)
        .await
        .map_err(|e| PortError::Io {
            message: format!(
                "Failed to check if project directory {:?} exists: {}",
                path, e
            ),
        })?;
    if exists {
        tokio::fs::remove_dir_all(&path)
            .await
            .map_err(|e| PortError::Io {
                message: format!("Failed to delete project directory {:?}: {}", path, e),
            })?;
    }
    Ok(())
}
