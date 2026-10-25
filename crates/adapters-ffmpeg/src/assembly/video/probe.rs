use super::super::{error, process};
use crate::ffprobe::{dto::FfprobeOutput, parser::parse_ffprobe_output};
use domain::media::MediaMetadata;
use ports::{PortError, cancellation::CancellationToken};
use std::{path::Path, time::Duration};
use tokio::process::Command;

pub(super) async fn probe(
    executable: &Path,
    source: &Path,
    cancel: CancellationToken,
) -> Result<MediaMetadata, PortError> {
    let mut command = Command::new(executable);
    command
        .args([
            "-v",
            "error",
            "-protocol_whitelist",
            "file,pipe",
            "-of",
            "json",
            "-show_format",
            "-show_streams",
        ])
        .arg(source);
    let bytes = process::capture(command, cancel, Duration::from_secs(30), 65536, 65536).await?;
    let output: FfprobeOutput = serde_json::from_slice(&bytes)
        .map_err(|_| error::invalid("Invalid bounded FFprobe output"))?;
    parse_ffprobe_output(&output).map_err(|error| super::super::error::invalid(error.to_string()))
}
