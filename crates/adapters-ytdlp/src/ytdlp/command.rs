use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

use super::error::YtDlpError;

pub async fn run_ytdlp_dump_json(
    candidates: &[PathBuf],
    url: &str,
    timeout_ms: u64,
) -> Result<String, YtDlpError> {
    for candidate in candidates {
        let child = match Command::new(candidate)
            .arg("--dump-json")
            .arg("--no-playlist")
            .arg(url)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
        {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                return Err(YtDlpError::SpawnFailed {
                    candidate: candidate.to_string_lossy().to_string(),
                    source: e,
                });
            }
        };

        let output =
            match timeout(Duration::from_millis(timeout_ms), child.wait_with_output()).await {
                Ok(Ok(out)) => out,
                Ok(Err(e)) => {
                    return Err(YtDlpError::SpawnFailed {
                        candidate: candidate.to_string_lossy().to_string(),
                        source: e,
                    });
                }
                Err(_) => {
                    return Err(YtDlpError::Timeout { timeout_ms });
                }
            };

        if !output.status.success() {
            return Err(YtDlpError::CommandFailed {
                code: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        let json = String::from_utf8(output.stdout).map_err(YtDlpError::InvalidUtf8)?;
        return Ok(json);
    }

    Err(YtDlpError::MissingYtDlp)
}
