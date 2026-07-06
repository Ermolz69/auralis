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
            YtDlpError::UnsupportedSource { message } => PortError::InvalidSource { message },
            _ => PortError::ExternalToolFailed {
                tool: "yt-dlp".to_string(),
                message: error.to_string(),
            },
        }
    }
}
