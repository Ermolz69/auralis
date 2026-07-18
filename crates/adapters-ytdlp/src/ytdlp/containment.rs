use ports::error::PortError;
use std::path::Path;

pub(crate) fn verify_containment(target_dir: &Path, path: &Path) -> Result<(), PortError> {
    let target_metadata = target_dir.symlink_metadata().map_err(|_| PortError::Io {
        message: "Target directory not found".to_string(),
    })?;

    if !target_metadata.is_dir() {
        return Err(PortError::Io {
            message: "Target is not a directory".to_string(),
        });
    }

    if target_metadata.is_symlink() {
        return Err(PortError::Io {
            message: "Target directory cannot be a symlink".to_string(),
        });
    }

    let output_metadata = path.symlink_metadata().map_err(|_| PortError::Io {
        message: "Output file not found".to_string(),
    })?;

    if output_metadata.is_symlink() {
        return Err(PortError::Io {
            message: "Output file cannot be a symlink".to_string(),
        });
    }

    if !output_metadata.is_file() {
        return Err(PortError::Io {
            message: "Output is not a regular file".to_string(),
        });
    }

    let canonical_target = target_dir.canonicalize().map_err(|_| PortError::Io {
        message: "Failed to canonicalize target directory".to_string(),
    })?;

    let canonical_path = path.canonicalize().map_err(|_| PortError::Io {
        message: "Failed to canonicalize output path".to_string(),
    })?;

    if canonical_path == canonical_target {
        return Err(PortError::Io {
            message: "Output path cannot be equal to target directory".to_string(),
        });
    }

    let remainder = canonical_path
        .strip_prefix(&canonical_target)
        .map_err(|_| PortError::InvalidSource {
            message: "Path traversal attempt detected".to_string(),
        })?;

    if remainder.as_os_str().is_empty() {
        return Err(PortError::Io {
            message: "Output path remainder cannot be empty".to_string(),
        });
    }

    let mut current = path;
    while let Some(parent) = current.parent() {
        if parent == target_dir || parent.as_os_str().is_empty() {
            break;
        }
        if current
            .symlink_metadata()
            .map(|m| m.is_symlink())
            .unwrap_or(false)
        {
            return Err(PortError::Io {
                message: "Symlink component in path is forbidden".to_string(),
            });
        }
        current = parent;
    }

    Ok(())
}
