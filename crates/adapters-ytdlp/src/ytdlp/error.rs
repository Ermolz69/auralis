use ports::error::PortError;

#[derive(Debug, thiserror::Error)]
pub enum YtDlpError {
    #[error("yt-dlp is not installed or not bundled. Run task setup:media-tools")]
    MissingYtDlp,

    #[error("failed to start yt-dlp candidate `{candidate}`: {source}")]
    SpawnFailed {
        candidate: String,
        #[source]
        source: std::io::Error,
    },

    #[error("yt-dlp timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    #[error("yt-dlp failed with exit code {code:?}: {stderr}")]
    CommandFailed { code: Option<i32>, stderr: String },

    #[error("yt-dlp returned invalid UTF-8 output")]
    InvalidUtf8(#[source] std::string::FromUtf8Error),

    #[error("failed to parse yt-dlp JSON: {0}")]
    ParseFailed(#[source] serde_json::Error),

    #[error("unsupported media source: {message}")]
    UnsupportedSource { message: String },

    #[error("subtitle file was not found after yt-dlp download")]
    SubtitleNotFoundAfterDownload,

    #[error("downloaded file does not exist: {path}")]
    DownloadedFileMissing { path: String },

    #[error("failed to create download directory `{path}`: {source}")]
    CreateDownloadDirFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

impl From<YtDlpError> for PortError {
    fn from(error: YtDlpError) -> Self {
        match error {
            YtDlpError::UnsupportedSource { .. } => PortError::InvalidSource {
                message: "Unsupported media source".to_string(),
            },
            YtDlpError::MissingYtDlp => PortError::ExternalToolFailed {
                tool: "yt-dlp".to_string(),
                message: "yt-dlp is not installed or not bundled".to_string(),
            },
            YtDlpError::SpawnFailed { .. } => PortError::ExternalToolFailed {
                tool: "yt-dlp".to_string(),
                message: "Failed to spawn yt-dlp process".to_string(),
            },
            YtDlpError::Timeout { .. } => PortError::ExternalToolFailed {
                tool: "yt-dlp".to_string(),
                message: "yt-dlp process timed out".to_string(),
            },
            YtDlpError::CommandFailed { .. } => PortError::ExternalToolFailed {
                tool: "yt-dlp".to_string(),
                message: "yt-dlp command failed during execution".to_string(),
            },
            YtDlpError::InvalidUtf8(_) => PortError::ExternalToolFailed {
                tool: "yt-dlp".to_string(),
                message: "yt-dlp returned invalid UTF-8 output".to_string(),
            },
            YtDlpError::ParseFailed(_) => PortError::ExternalToolFailed {
                tool: "yt-dlp".to_string(),
                message: "Failed to parse yt-dlp metadata".to_string(),
            },
            YtDlpError::SubtitleNotFoundAfterDownload => PortError::ExternalToolFailed {
                tool: "yt-dlp".to_string(),
                message: "Subtitle file was not found after download".to_string(),
            },
            YtDlpError::DownloadedFileMissing { .. } => PortError::ExternalToolFailed {
                tool: "yt-dlp".to_string(),
                message: "Downloaded file is missing".to_string(),
            },
            YtDlpError::CreateDownloadDirFailed { .. } => PortError::ExternalToolFailed {
                tool: "yt-dlp".to_string(),
                message: "Failed to create download directory".to_string(),
            },
        }
    }
}
