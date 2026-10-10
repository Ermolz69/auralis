use ports::{PortError, cancellation::CancellationToken};
use std::{process::Output, time::Duration};
use tokio::io::AsyncReadExt;

#[derive(Debug, Clone, Copy)]
pub struct ProcessLimits {
    pub execution_timeout: Duration,
    pub stop_timeout: Duration,
    pub output_bytes: u64,
    pub exit_poll_interval: Duration,
}

impl Default for ProcessLimits {
    fn default() -> Self {
        Self {
            execution_timeout: Duration::from_secs(60),
            stop_timeout: Duration::from_secs(7),
            output_bytes: 4 * 1024 * 1024,
            exit_poll_interval: Duration::from_millis(25),
        }
    }
}

impl ProcessLimits {
    pub fn validate(self) -> Result<(), PortError> {
        if self.execution_timeout.is_zero()
            || self.execution_timeout > Duration::from_secs(1800)
            || self.stop_timeout.is_zero()
            || self.stop_timeout > Duration::from_secs(300)
            || self.output_bytes == 0
            || self.output_bytes > 16 * 1024 * 1024
            || self.exit_poll_interval.is_zero()
            || self.exit_poll_interval > Duration::from_secs(1)
        {
            return Err(PortError::InvalidRuntimeConfiguration {
                message: "Invalid subtitle process limits".into(),
            });
        }
        Ok(())
    }
}

pub(crate) async fn wait(
    mut child: super::OwnedChild,
    token: &CancellationToken,
    limits: ProcessLimits,
) -> Result<Output, PortError> {
    limits.validate()?;
    let stdout = child.stdout().take();
    let stderr = child.stderr().take();
    let wait = async {
        let deadline = tokio::time::Instant::now() + limits.execution_timeout;
        let (result, interrupted) = loop {
            match child.try_wait() {
                Ok(Some(status)) => break (Ok(status), false),
                Err(error) => break (Err(error), true),
                Ok(None) => {}
            }
            tokio::select! {
                _ = token.cancelled() => break (Err(std::io::Error::other("cancelled")), true),
                _ = tokio::time::sleep_until(deadline) => break (Err(std::io::Error::other("process deadline exceeded")), true),
                _ = tokio::time::sleep(limits.exit_poll_interval) => {},
            }
        };
        // Wait for the tree exactly once. Dropping process-wrap's first wait can cache
        // the parent exit before descendants stop; a second wait would then skip them.
        child
            .start_kill()
            .map_err(|error| PortError::CleanupFailed {
                resource: "yt-dlp process tree".into(),
                message: error.to_string(),
            })?;
        let stopped = async {
            child.wait().await?;
            #[cfg(unix)]
            while !child.tree_exited()? {
                tokio::time::sleep(limits.exit_poll_interval).await;
            }
            Ok::<(), std::io::Error>(())
        };
        if !matches!(
            tokio::time::timeout(limits.stop_timeout, stopped).await,
            Ok(Ok(()))
        ) {
            return Err(PortError::CleanupFailed {
                resource: "yt-dlp process tree".into(),
                message: "Process exit was not confirmed".into(),
            });
        }
        if interrupted || result.is_err() {
            return Err(if token.is_cancelled() {
                PortError::Cancelled
            } else {
                PortError::ExternalToolFailed {
                    tool: "yt-dlp".into(),
                    message: "Process timed out or failed to wait".into(),
                }
            });
        }
        result.map_err(|error| PortError::Io {
            message: error.to_string(),
        })
    };
    let joined = async {
        tokio::join!(
            wait,
            read(stdout, limits.output_bytes),
            read(stderr, limits.output_bytes)
        )
    };
    let (status, stdout, stderr) =
        tokio::time::timeout(limits.execution_timeout + limits.stop_timeout, joined)
            .await
            .map_err(|_| PortError::CleanupFailed {
                resource: "yt-dlp process tree".into(),
                message: "Process or pipe cleanup is unconfirmed".into(),
            })?;
    let status = status?;
    Ok(Output {
        status,
        stdout: stdout?,
        stderr: stderr?,
    })
}

async fn read<R: tokio::io::AsyncRead + Unpin>(
    pipe: Option<R>,
    limit: u64,
) -> Result<Vec<u8>, PortError> {
    let mut output = Vec::new();
    if let Some(pipe) = pipe {
        pipe.take(limit + 1)
            .read_to_end(&mut output)
            .await
            .map_err(|error| PortError::Io {
                message: error.to_string(),
            })?;
    }
    if output.len() as u64 > limit {
        return Err(PortError::ExternalToolFailed {
            tool: "yt-dlp".into(),
            message: "Process output limit exceeded".into(),
        });
    }
    Ok(output)
}
