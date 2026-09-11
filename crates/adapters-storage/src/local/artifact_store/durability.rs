use ports::error::PortError;
use std::path::Path;

pub(super) async fn sync_file(
    base: &Path,
    path: &Path,
    expected_size: Option<u64>,
) -> Result<(), PortError> {
    let metadata = tokio::fs::symlink_metadata(path).await.map_err(io_error)?;
    if !metadata.is_file() || expected_size.is_some_and(|size| metadata.len() != size) {
        return Err(PortError::Conflict {
            resource: "Artifact".into(),
            message: "Artifact file type or size does not match the committed intent".into(),
        });
    }
    let file = tokio::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .await
        .map_err(io_error)?;
    file.sync_all().await.map_err(io_error)?;
    sync_directories(base, path).await
}

#[cfg(unix)]
pub(super) async fn sync_directories(base: &Path, path: &Path) -> Result<(), PortError> {
    let base = base.canonicalize().map_err(io_error)?;
    let parent = path
        .parent()
        .ok_or_else(|| io_error(std::io::Error::other("Missing artifact parent")))?
        .canonicalize()
        .map_err(io_error)?;
    tokio::task::spawn_blocking(move || {
        for directory in parent
            .ancestors()
            .take_while(|directory| directory.starts_with(&base))
        {
            std::fs::File::open(directory)?.sync_all()?;
        }
        Ok::<_, std::io::Error>(())
    })
    .await
    .map_err(|_| io_error(std::io::Error::other("Artifact directory sync interrupted")))?
    .map_err(io_error)
}

#[cfg(not(unix))]
pub(super) async fn sync_directories(_base: &Path, _path: &Path) -> Result<(), PortError> {
    // Windows std does not expose a durable directory flush; file data is flushed separately.
    Ok(())
}

fn io_error(error: std::io::Error) -> PortError {
    PortError::Storage {
        operation: "sync_artifact",
        message: error.to_string(),
    }
}
