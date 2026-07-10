use domain::media::{Artifact, ArtifactId, ArtifactKind};
use domain::project::ProjectId;
use ports::error::PortError;
use std::path::Path;

use super::path_guard::ensure_safe_parent;
use super::resolver::resolve_storage_key;
use crate::local::storage_key::make_storage_key;

pub async fn write_small_artifact(
    base_dir: &Path,
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
    let path = resolve_storage_key(base_dir, &storage_key)?;

    ensure_safe_parent(base_dir, &path).await?;

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
