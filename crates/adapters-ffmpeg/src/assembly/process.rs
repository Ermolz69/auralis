use super::error;
use common::owned_process;
use ports::{PortError, cancellation::CancellationToken};
use std::{process::Stdio, time::Duration};
use tokio::{io::AsyncReadExt, process::Command};
#[cfg(test)]
mod tests;

pub(super) async fn run(
    mut command: Command,
    cancel: CancellationToken,
    deadline: Duration,
    stderr_limit: usize,
) -> Result<(), PortError> {
    if cancel.is_cancelled() {
        return Err(PortError::Cancelled);
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let mut child = owned_process::spawn(command).map_err(error::io)?;
    let Some(stderr) = child.take_stderr() else {
        child.stop().await.map_err(error::io)?;
        return Err(error::external("FFmpeg stderr pipe is missing"));
    };
    let result = {
        let operation = async {
            let wait = async { child.wait().await.map_err(error::io) };
            let read = async {
                let mut bytes = Vec::new();
                stderr
                    .take(stderr_limit as u64 + 1)
                    .read_to_end(&mut bytes)
                    .await
                    .map_err(error::io)?;
                if bytes.len() > stderr_limit {
                    return Err(error::external("FFmpeg stderr byte limit exceeded"));
                }
                Ok(bytes)
            };
            let (status, stderr) = tokio::try_join!(wait, read)?;
            if !status.success() {
                return Err(error::external(format!(
                    "FFmpeg exited {status}: {}",
                    String::from_utf8_lossy(&stderr)
                )));
            }
            Ok(())
        };
        tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(PortError::Cancelled),
            _ = tokio::time::sleep(deadline) => Err(error::external("FFmpeg process deadline exceeded")),
            result = operation => result,
        }
    };
    if result.is_err() {
        match tokio::time::timeout(Duration::from_secs(5), child.stop()).await {
            Ok(Ok(())) => {}
            _ => {
                return Err(PortError::CleanupFailed {
                    resource: "ffmpeg_process".into(),
                    message: "Owned process exit was not confirmed".into(),
                });
            }
        }
    }
    result
}
