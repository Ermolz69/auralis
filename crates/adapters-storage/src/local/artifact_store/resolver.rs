use domain::media::Artifact;
use ports::error::PortError;
use std::path::{Path, PathBuf};

use super::path_guard::verify_path_under_base_dir;

pub fn resolve_storage_key(base_dir: &Path, key: &str) -> Result<PathBuf, PortError> {
    let key_path = PathBuf::from(key);

    if key_path.components().any(|c| {
        matches!(
            c,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        )
    }) {
        return Err(PortError::Unexpected {
            message: "StorageKey must be a clean relative path".into(),
        });
    }

    let full_path = base_dir.join(key_path);
    verify_path_under_base_dir(base_dir, &full_path)?;

    Ok(full_path)
}

pub fn resolve_artifact(base_dir: &Path, artifact: &Artifact) -> Result<PathBuf, PortError> {
    match &artifact.location {
        domain::media::ArtifactLocation::LocalPath(_) => Err(PortError::Unsupported {
            message: "Only managed StorageKey artifact locations can be resolved".to_string(),
        }),
        domain::media::ArtifactLocation::StorageKey(key) => resolve_storage_key(base_dir, key),
    }
}
