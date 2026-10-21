use ports::error::PortError;
use std::path::Path;

// Called only after the owned CLI process tree has exited while the PUZA lease is held.
pub async fn release_owned(root: &Path, process_id: Option<u32>) -> Result<(), PortError> {
    let Some(process_id) = process_id else {
        return Ok(());
    };
    let lab = root.parent().map(|parent| parent.join("Algorithm Lab"));
    let directory = lab
        .filter(|path| path.is_dir())
        .unwrap_or_else(|| root.to_path_buf());
    let lock = directory.join(".normalizer.lock");
    let contents = match tokio::fs::read_to_string(&lock).await {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(invalid(error)),
    };
    if contents.trim() == process_id.to_string() {
        tokio::fs::remove_file(lock).await.map_err(invalid)?;
    }
    Ok(())
}

fn invalid(error: std::io::Error) -> PortError {
    PortError::InvalidSource {
        message: format!("Не удалось освободить блокировку маркировщика: {error}"),
    }
}
