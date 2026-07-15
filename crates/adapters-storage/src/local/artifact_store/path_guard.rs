use ports::error::PortError;
use std::path::Path;

pub async fn ensure_safe_parent(base_dir: &Path, path: &Path) -> Result<(), PortError> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| PortError::Io {
                message: format!("Failed to create directory {:?}: {}", parent, e),
            })?;

        let canon_parent = parent.canonicalize().map_err(|e| PortError::Io {
            message: format!("Failed to canonicalize parent {:?}: {}", parent, e),
        })?;

        let canon_base = base_dir
            .canonicalize()
            .unwrap_or_else(|_| base_dir.to_path_buf());
        if canon_parent.strip_prefix(&canon_base).is_err() {
            return Err(PortError::Io {
                message: "Resolved artifact path escapes root".to_string(),
            });
        }
    }
    Ok(())
}

pub fn verify_path_under_base_dir(base_dir: &Path, path: &Path) -> Result<(), PortError> {
    let mut current = path.to_path_buf();
    while !current.exists() {
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            break;
        }
    }

    let canon_current = current.canonicalize().map_err(|e| PortError::Io {
        message: format!("Failed to canonicalize path {:?}: {}", current, e),
    })?;

    let canon_base = base_dir
        .canonicalize()
        .unwrap_or_else(|_| base_dir.to_path_buf());

    if canon_current.strip_prefix(&canon_base).is_err() {
        return Err(PortError::Io {
            message: "Path escapes base directory".to_string(),
        });
    }
    Ok(())
}
