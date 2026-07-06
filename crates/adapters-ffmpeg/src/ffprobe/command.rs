use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;

use crate::ffprobe::dto::FfprobeOutput;
use crate::ffprobe::error::FfprobeError;

pub async fn run_ffprobe(
    candidates: &[PathBuf],
    file_path: &Path,
) -> Result<FfprobeOutput, FfprobeError> {
    for candidate in candidates {
        let output_res = Command::new(candidate)
            .arg("-v")
            .arg("error")
            .arg("-print_format")
            .arg("json")
            .arg("-show_format")
            .arg("-show_streams")
            .arg(file_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        match output_res {
            Ok(output) => {
                if !output.status.success() {
                    let stderr_str = String::from_utf8_lossy(&output.stderr);
                    return Err(FfprobeError::ProcessError(stderr_str.to_string()));
                }

                let parsed: FfprobeOutput = serde_json::from_slice(&output.stdout)?;
                return Ok(parsed);
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    // Try next candidate
                    continue;
                } else {
                    return Err(FfprobeError::CommandFailed(e));
                }
            }
        }
    }

    Err(FfprobeError::MissingFfprobe)
}
