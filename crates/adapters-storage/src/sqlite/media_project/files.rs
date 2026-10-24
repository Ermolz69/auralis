use super::invalid;
use ports::error::PortError;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(super) fn identity(id: &str) -> Result<(), PortError> {
    uuid::Uuid::parse_str(id).map_err(invalid)?;
    Ok(())
}
pub(super) fn section(kind: &domain::library::LibraryAssetKind) -> &'static str {
    use domain::library::LibraryAssetKind::*;
    match kind {
        Text => "text",
        Subtitle => "subtitles",
        Audio => "audio",
        Video => "video",
    }
}
pub(super) fn safe_path(root: &Path, key: &str) -> Result<PathBuf, PortError> {
    if key.contains(['\\', ':'])
        || key
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == ".." || p.ends_with(['.', ' ']))
    {
        return Err(invalid("Небезопасный путь материала"));
    }
    let mut result = root.to_path_buf();
    for part in key.split('/') {
        result.push(part);
        match std::fs::symlink_metadata(&result) {
            Ok(info) => {
                #[cfg(windows)]
                let linked = {
                    use std::os::windows::fs::MetadataExt;
                    info.file_attributes() & 0x400 != 0
                };
                #[cfg(not(windows))]
                let linked = info.file_type().is_symlink();
                if linked {
                    return Err(invalid("Ссылки и junction запрещены в структуре проекта"));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(invalid(e)),
        }
    }
    Ok(result)
}
pub(super) async fn digest(path: &Path) -> Result<(u64, String), PortError> {
    let mut file = tokio::fs::File::open(path).await.map_err(invalid)?;
    let mut hash = Sha256::new();
    let mut size = 0;
    let mut bytes = vec![0; 1024 * 1024];
    loop {
        let count = file.read(&mut bytes).await.map_err(invalid)?;
        if count == 0 {
            break;
        }
        hash.update(&bytes[..count]);
        size += count as u64;
    }
    Ok((size, format!("{:x}", hash.finalize())))
}
pub(super) async fn atomic(path: &Path, bytes: &[u8]) -> Result<(), PortError> {
    let parent = path
        .parent()
        .ok_or_else(|| invalid("Путь без родительской папки"))?;
    tokio::fs::create_dir_all(parent).await.map_err(invalid)?;
    let temporary = parent.join(format!(".{}.pending", uuid::Uuid::new_v4()));
    let mut file = tokio::fs::File::create(&temporary).await.map_err(invalid)?;
    file.write_all(bytes).await.map_err(invalid)?;
    file.sync_all().await.map_err(invalid)?;
    drop(file);
    tokio::fs::rename(&temporary, path).await.map_err(invalid)
}
pub(super) async fn copy_verified(
    source: &Path,
    destination: &Path,
) -> Result<(u64, String), PortError> {
    let expected = digest(source).await?;
    if destination.is_file() && digest(destination).await? == expected {
        return Ok(expected);
    }
    if destination.exists() {
        return Err(invalid("Существующий файл проекта отличается от источника"));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| invalid("Путь без папки"))?;
    tokio::fs::create_dir_all(parent).await.map_err(invalid)?;
    let temp = parent.join(format!(".{}.copy", uuid::Uuid::new_v4()));
    tokio::fs::copy(source, &temp).await.map_err(invalid)?;
    if digest(&temp).await? != expected {
        let _ = tokio::fs::remove_file(&temp).await;
        return Err(invalid("Файл изменился во время копирования"));
    }
    tokio::fs::rename(temp, destination)
        .await
        .map_err(invalid)?;
    Ok(expected)
}
