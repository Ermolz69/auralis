use std::path::Path;
use tokio::io::AsyncWriteExt;

pub(super) async fn copy_external(source: &Path, destination: &Path) -> std::io::Result<u64> {
    let mut input = tokio::fs::File::open(source).await?;
    let mut output = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .await?;
    let size = tokio::io::copy(&mut input, &mut output).await?;
    output.flush().await?;
    output.sync_all().await?;
    Ok(size)
}
