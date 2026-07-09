use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;

use crate::ffprobe::dto::FfprobeOutput;
use crate::ffprobe::error::FfprobeError;

use std::time::Duration;
use tokio::time::timeout;

pub async fn run_ffprobe(
    candidates: &[PathBuf],
    file_path: &Path,
    timeout_ms: u64,
) -> Result<FfprobeOutput, FfprobeError> {
    for candidate in candidates {
        let child = match Command::new(candidate)
            .arg("-v")
            .arg("error")
            .arg("-print_format")
            .arg("json")
            .arg("-show_format")
            .arg("-show_streams")
            .arg(file_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
        {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                return Err(FfprobeError::SpawnFailed {
                    candidate: candidate.to_string_lossy().to_string(),
                    source: e,
                });
            }
        };

        let output =
            match timeout(Duration::from_millis(timeout_ms), child.wait_with_output()).await {
                Ok(Ok(out)) => out,
                Ok(Err(e)) => {
                    return Err(FfprobeError::SpawnFailed {
                        candidate: candidate.to_string_lossy().to_string(),
                        source: e,
                    });
                }
                Err(_) => {
                    return Err(FfprobeError::Timeout { timeout_ms });
                }
            };

        if !output.status.success() {
            let stderr_str = String::from_utf8_lossy(&output.stderr);
            return Err(FfprobeError::ProcessError(stderr_str.to_string()));
        }

        let parsed: FfprobeOutput = serde_json::from_slice(&output.stdout)?;
        return Ok(parsed);
    }

    Err(FfprobeError::MissingFfprobe)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_ffprobe_timeout() {
        // Ensure we test timeout logic by passing 0ms.
        // On any system, spawning should take longer than 0ms or hit timeout immediately.
        let candidates = vec![PathBuf::from("ffprobe")];
        let file_path = PathBuf::from("dummy.mp4");

        let res = run_ffprobe(&candidates, &file_path, 0).await;

        match res {
            Err(FfprobeError::Timeout { timeout_ms }) => assert_eq!(timeout_ms, 0),
            Err(FfprobeError::MissingFfprobe) => { /* that's fine if not installed */ }
            Err(FfprobeError::SpawnFailed { .. }) => { /* also fine if no binary */ }
            _ => panic!("Expected Timeout or Missing/SpawnFailed, got {:?}", res),
        }
    }
}
