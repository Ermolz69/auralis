#![allow(clippy::unwrap_used, clippy::expect_used)]
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
        let filepath = extract_final_filepath_from_stdout(&stdout_str).unwrap_or_default(); // allow-fallback

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

pub async fn run_ytdlp_download_subtitle(
    candidates: &[PathBuf],
    url: &str,
    target_dir: &std::path::Path,
    language: &str,
    format: &str,
    auto_generated: bool,
    timeout_ms: u64,
) -> Result<PathBuf, YtDlpError> {
    tokio::fs::create_dir_all(target_dir).await.map_err(|e| {
        YtDlpError::CreateDownloadDirFailed {
            path: target_dir.to_string_lossy().to_string(),
            source: e,
        }
    })?;

    for candidate in candidates {
        let run_id = uuid::Uuid::new_v4().to_string();
        let mut command = Command::new(candidate);

        command
            .arg("--skip-download")
            .arg("--no-playlist")
            .arg("--no-warnings")
            .arg("--windows-filenames")
            .arg("--sub-langs")
            .arg(language)
            .arg("--sub-format")
            .arg(format)
            .arg("-P")
            .arg(target_dir)
            .arg("-o")
            .arg(format!("{}.%(ext)s", run_id));

        if auto_generated {
            command.arg("--write-auto-sub");
        } else {
            command.arg("--write-sub");
        }

        command
            .arg(url)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let child = match command.spawn() {
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
                Err(_) => return Err(YtDlpError::Timeout { timeout_ms }),
            };

        if !output.status.success() {
            return Err(YtDlpError::CommandFailed {
                code: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        if let Some(path) = find_downloaded_subtitle(target_dir, &run_id, format).await? {
            return Ok(path);
        }

        return Err(YtDlpError::SubtitleNotFoundAfterDownload);
    }

    Err(YtDlpError::MissingYtDlp)
}

#[allow(clippy::collapsible_if)]
async fn find_downloaded_subtitle(
    dir: &std::path::Path,
    run_id: &str,
    expected_format: &str,
) -> Result<Option<PathBuf>, YtDlpError> {
    let mut entries = tokio::fs::read_dir(dir)
        .await
        .map_err(|_| YtDlpError::SubtitleNotFoundAfterDownload)?;
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.is_file() {
            let file_name = path.file_name().unwrap_or_default().to_string_lossy(); // allow-fallback
            if file_name.starts_with(run_id)
                && file_name.ends_with(&format!(".{}", expected_format))
            {
                return Ok(Some(path));
            }
        }
    }
    Ok(None)
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
