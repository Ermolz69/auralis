use ports::error::PortError;
use std::path::Path;

pub async fn cleanup_stale_staging(
    base_dir: &Path,
    max_age: std::time::Duration,
) -> Result<(), PortError> {
    let staging_dir = base_dir.join(".staging");
    if !tokio::fs::try_exists(&staging_dir).await.unwrap_or(false) {
        return Ok(());
    }

    let mut entries = tokio::fs::read_dir(&staging_dir)
        .await
        .map_err(|e| PortError::Io {
            message: format!("Failed to read .staging directory: {}", e),
        })?;

    let now = std::time::SystemTime::now();

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.is_dir()
            && let Ok(metadata) = entry.metadata().await
            && let Ok(modified) = metadata.modified()
            && let Ok(age) = now.duration_since(modified)
            && age > max_age
        {
            let _ = tokio::fs::remove_dir_all(&path).await;
        }
    }
    Ok(())
}
