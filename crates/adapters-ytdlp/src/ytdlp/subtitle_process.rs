use ports::{PortError, cancellation::CancellationToken};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::process::Command;

use super::{command::YtDlpCommandPaths, owned_process::cancellable::ProcessLimits};

pub(super) async fn metadata(
    paths: YtDlpCommandPaths<'_>,
    url: &str,
    token: &CancellationToken,
    limits: ProcessLimits,
) -> Result<String, PortError> {
    let output = run(paths, url, &["--dump-json", "--no-playlist"], token, limits).await?;
    String::from_utf8(output).map_err(|e| PortError::ExternalToolFailed {
        tool: "yt-dlp".into(),
        message: e.to_string(),
    })
}

pub(super) async fn download(
    paths: YtDlpCommandPaths<'_>,
    url: &str,
    target: &Path,
    track: &domain::media::SubtitleTrack,
    token: &CancellationToken,
    limits: ProcessLimits,
) -> Result<PathBuf, PortError> {
    let id = uuid::Uuid::new_v4().to_string();
    let template = format!("{id}.%(ext)s");
    let target_arg = target.to_string_lossy();
    let kind = if track.is_auto_generated {
        "--write-auto-sub"
    } else {
        "--write-sub"
    };
    tokio::fs::create_dir_all(target)
        .await
        .map_err(|e| PortError::Io {
            message: e.to_string(),
        })?;
    run(
        paths,
        url,
        &[
            "--skip-download",
            "--no-playlist",
            "--no-warnings",
            "--windows-filenames",
            "--sub-langs",
            &track.language,
            "--sub-format",
            "vtt",
            "-P",
            &target_arg,
            "-o",
            &template,
            kind,
        ],
        token,
        limits,
    )
    .await?;
    super::command::find_downloaded_subtitle(target, &id, "vtt")
        .await?
        .ok_or_else(|| super::error::YtDlpError::SubtitleNotFoundAfterDownload.into())
}

async fn run(
    paths: YtDlpCommandPaths<'_>,
    url: &str,
    args: &[&str],
    token: &CancellationToken,
    limits: ProcessLimits,
) -> Result<Vec<u8>, PortError> {
    limits.validate()?;
    for candidate in paths.ytdlp_candidates {
        if token.is_cancelled() {
            return Err(PortError::Cancelled);
        }
        let mut command = Command::new(candidate);
        command.arg("--ignore-config").args(args);
        super::command::apply_ffmpeg_location(&mut command, paths.ffmpeg_candidates);
        command
            .arg(url)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let child = match super::owned_process::spawn(command) {
            Ok(child) => child,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                return Err(super::error::YtDlpError::SpawnFailed {
                    candidate: candidate.to_string_lossy().into(),
                    source: e,
                }
                .into());
            }
        };
        let output = super::owned_process::cancellable::wait(child, token, limits).await?;
        #[cfg(unix)]
        if output.status.code() == Some(127) {
            continue;
        }
        if !output.status.success() {
            return Err(super::error::YtDlpError::CommandFailed {
                code: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).into(),
            }
            .into());
        }
        return Ok(output.stdout);
    }
    Err(super::error::YtDlpError::MissingYtDlp.into())
}
