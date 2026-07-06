use thiserror::Error;

#[derive(Debug, Error)]
pub enum FfprobeError {
    #[error("Failed to execute ffprobe command: {0}")]
    CommandFailed(#[from] std::io::Error),

    #[error("ffprobe process exited with error status: {0}")]
    ProcessError(String),

    #[error("Failed to parse ffprobe JSON output: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("Missing expected format data in ffprobe output")]
    MissingFormatData,

    #[error("ffprobe is not installed or not bundled. Run task setup:media-tools")]
    MissingFfprobe,
}

impl From<FfprobeError> for ports::error::PortError {
    fn from(err: FfprobeError) -> Self {
        ports::error::PortError::ExternalToolFailed {
            tool: "ffprobe".to_string(),
            message: err.to_string(),
        }
    }
}
