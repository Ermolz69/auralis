use async_trait::async_trait;
use std::path::PathBuf;
use url::Url;

use domain::media::{MediaMetadata, MediaSource};
use ports::error::PortError;
use ports::source::VideoSourcePort;

use super::command::run_ytdlp_dump_json;
use super::parser::parse_ytdlp_metadata;

pub struct YtDlpAdapter {
    candidates: Vec<PathBuf>,
    timeout_ms: u64,
}

impl Default for YtDlpAdapter {
    fn default() -> Self {
        Self::new(vec![PathBuf::from("yt-dlp"), PathBuf::from("yt-dlp.exe")])
    }
}

impl YtDlpAdapter {
    pub fn new(candidates: Vec<PathBuf>) -> Self {
        Self {
            candidates,
            timeout_ms: 60_000,
        }
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

#[async_trait]
impl VideoSourcePort for YtDlpAdapter {
    async fn validate_source(&self, source: &MediaSource) -> Result<(), PortError> {
        let url_str = match source {
            MediaSource::YoutubeUrl { url } => url,
            MediaSource::RemoteUrl { url } => url,
            MediaSource::LocalFile { .. } => {
                return Err(PortError::InvalidSource {
                    message: "yt-dlp adapter only supports URLs, not local files".to_string(),
                });
            }
        };

        let parsed = Url::parse(url_str).map_err(|_| PortError::InvalidSource {
            message: format!("invalid URL: {}", url_str),
        })?;

        let host = parsed.host_str().unwrap_or("");
        if !host.contains("youtube.com") && !host.contains("youtu.be") {
            return Err(PortError::InvalidSource {
                message: "only youtube.com / youtu.be are supported right now".to_string(),
            });
        }

        Ok(())
    }

    async fn fetch_metadata(&self, source: &MediaSource) -> Result<MediaMetadata, PortError> {
        self.validate_source(source).await?;

        let url = match source {
            MediaSource::YoutubeUrl { url } => url,
            MediaSource::RemoteUrl { url } => url,
            _ => unreachable!(),
        };

        let json = run_ytdlp_dump_json(&self.candidates, url, self.timeout_ms).await?;
        let metadata = parse_ytdlp_metadata(&json)?;

        Ok(metadata)
    }

    async fn download_media(
        &self,
        _request: ports::source::DownloadMediaRequest,
    ) -> Result<domain::media::Artifact, PortError> {
        Err(PortError::Unsupported {
            message: "yt-dlp download is not implemented yet".to_string(),
        })
    }
}
