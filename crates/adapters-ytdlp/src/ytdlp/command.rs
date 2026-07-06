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

pub async fn run_ytdlp_download(
    candidates: &[PathBuf],
    url: &str,
    target_dir: &std::path::Path,
    filename_template: &str,
    timeout_ms: u64,
) -> Result<PathBuf, YtDlpError> {
    for candidate in candidates {
        let child = match Command::new(candidate)
            .arg("--no-playlist")
            .arg("--no-warnings")
            .arg("--windows-filenames")
            .arg("--print")
            .arg("after_move:filepath")
            .arg("-P")
            .arg(target_dir)
            .arg("-o")
            .arg(filename_template)
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

        let stdout_str = String::from_utf8(output.stdout).map_err(YtDlpError::InvalidUtf8)?;
        let filepath = extract_final_filepath_from_stdout(&stdout_str).unwrap_or_default();

        if filepath.is_empty() {
            return Err(YtDlpError::CommandFailed {
                code: Some(0),
                stderr: "yt-dlp returned empty filepath".to_string(),
            });
        }

        return Ok(PathBuf::from(filepath));
    }

    Err(YtDlpError::MissingYtDlp)
}

pub fn extract_final_filepath_from_stdout(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .rfind(|l| !l.trim().is_empty())
        .map(|l| l.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_final_filepath_from_stdout() {
        assert_eq!(
            extract_final_filepath_from_stdout("some warning\n/path/to/video.mp4\n"),
            Some("/path/to/video.mp4".to_string())
        );
        assert_eq!(
            extract_final_filepath_from_stdout("/path/to/video.mp4"),
            Some("/path/to/video.mp4".to_string())
        );
        assert_eq!(
            extract_final_filepath_from_stdout("   /path/to/video.mp4   "),
            Some("/path/to/video.mp4".to_string())
        );
        assert_eq!(extract_final_filepath_from_stdout("   \n   \n"), None);
        assert_eq!(extract_final_filepath_from_stdout(""), None);
    }
}
