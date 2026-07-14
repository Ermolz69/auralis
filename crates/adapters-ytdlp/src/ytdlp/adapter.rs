use async_trait::async_trait;
use std::path::PathBuf;
use url::Url;

use domain::media::{MediaMetadata, MediaSource};
use ports::error::PortError;
use ports::source::VideoSourcePort;

use super::command::run_ytdlp_dump_json;
use super::parser::parse_ytdlp_metadata;

#[derive(Clone)]
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
            MediaSource::ManagedLocalFile { .. } | MediaSource::ExternalLocalFile { .. } => {
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
        request: ports::source::DownloadMediaRequest,
    ) -> Result<domain::media::Artifact, PortError> {
        self.validate_source(&request.source).await?;

        let url = match &request.source {
            MediaSource::YoutubeUrl { url } => url,
            MediaSource::RemoteUrl { url } => url,
            _ => unreachable!(),
        };

        if !request.target_dir.exists() {
            tokio::fs::create_dir_all(&request.target_dir)
                .await
                .map_err(|e| super::error::YtDlpError::CreateDownloadDirFailed {
                    path: request.target_dir.to_string_lossy().to_string(),
                    source: e,
                })?;
        }

        let template = build_output_template(request.filename_hint.as_deref());

        let path = super::command::run_ytdlp_download(
            &self.candidates,
            url,
            &request.target_dir,
            &template,
            self.timeout_ms,
        )
        .await?;

        if !path.exists() {
            return Err(super::error::YtDlpError::DownloadedFileMissing {
                path: path.to_string_lossy().to_string(),
            }
            .into());
        }

        Ok(domain::media::Artifact {
            id: domain::media::ArtifactId(uuid::Uuid::new_v4()),
            kind: domain::media::ArtifactKind::DownloadedVideo,
            location: domain::media::ArtifactLocation::LocalPath(
                path.to_string_lossy().to_string(),
            ),
            size_bytes: None,
            state: domain::media::ArtifactState::Ready,
            created_at: domain::chrono::Utc::now(),
            updated_at: domain::chrono::Utc::now(),
            ready_at: Some(domain::chrono::Utc::now()),
        })
    }
}

use super::parser::parse_subtitle_tracks;
use domain::media::{ArtifactKind, ArtifactLocation, SubtitleTrack};
use ports::source::SubtitleSourcePort;

#[async_trait]
impl SubtitleSourcePort for YtDlpAdapter {
    async fn list_subtitles(&self, source: &MediaSource) -> Result<Vec<SubtitleTrack>, PortError> {
        self.validate_source(source).await?;

        let url = match source {
            MediaSource::YoutubeUrl { url } => url,
            MediaSource::RemoteUrl { url } => url,
            MediaSource::ManagedLocalFile { .. } | MediaSource::ExternalLocalFile { .. } => {
                return Err(PortError::InvalidSource {
                    message: "yt-dlp subtitles only support URLs".to_string(),
                });
            }
        };

        let json = run_ytdlp_dump_json(&self.candidates, url, self.timeout_ms).await?;
        let value: serde_json::Value =
            serde_json::from_str(&json).map_err(super::error::YtDlpError::ParseFailed)?;

        Ok(parse_subtitle_tracks(&value))
    }

    async fn download_subtitle(
        &self,
        source: &MediaSource,
        track: &SubtitleTrack,
        target_path: &std::path::Path,
    ) -> Result<domain::media::Artifact, PortError> {
        self.validate_source(source).await?;

        let url = match source {
            MediaSource::YoutubeUrl { url } => url,
            MediaSource::RemoteUrl { url } => url,
            MediaSource::ManagedLocalFile { .. } | MediaSource::ExternalLocalFile { .. } => {
                return Err(PortError::InvalidSource {
                    message: "yt-dlp subtitle download only supports URLs".to_string(),
                });
            }
        };

        let path = super::command::run_ytdlp_download_subtitle(
            &self.candidates,
            url,
            target_path,
            &track.language,
            "vtt",
            track.is_auto_generated,
            self.timeout_ms,
        )
        .await?;

        Ok(domain::media::Artifact {
            id: domain::media::ArtifactId::new(),
            kind: ArtifactKind::OriginalSubtitle,
            location: ArtifactLocation::LocalPath(path.to_string_lossy().to_string()),
            size_bytes: None,
            state: domain::media::ArtifactState::Ready,
            created_at: domain::chrono::Utc::now(),
            updated_at: domain::chrono::Utc::now(),
            ready_at: Some(domain::chrono::Utc::now()),
        })
    }
}
fn build_output_template(filename_hint: Option<&str>) -> String {
    if let Some(hint) = filename_hint {
        let sanitized = sanitize_filename(hint);
        format!("{}.%(ext)s", sanitized)
    } else {
        "%(title).120B [%(id)s].%(ext)s".to_string()
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::media::ArtifactKind;
    use std::env;
    use tokio::fs;

    #[test]
    fn test_build_output_template_uses_hint() {
        assert_eq!(
            build_output_template(Some("My <Video> Name!")),
            "My _Video_ Name!.%(ext)s"
        );
    }

    #[test]
    fn test_build_output_template_uses_default() {
        assert_eq!(
            build_output_template(None),
            "%(title).120B [%(id)s].%(ext)s"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn test_downloads_public_test_video() {
        let adapter = YtDlpAdapter::default();
        let target_dir = env::temp_dir().join(format!("auralis_test_{}", uuid::Uuid::new_v4()));
        let source = MediaSource::RemoteUrl {
            url: "https://www.youtube.com/watch?v=jNQXAC9IVRw".to_string(), // "Me at the zoo" - shortest video
        };

        let request = ports::source::DownloadMediaRequest {
            source,
            target_dir: target_dir.clone(),
            filename_hint: Some("test_video".to_string()),
        };

        let artifact = adapter
            .download_media(request)
            .await
            .expect("Download failed");

        assert_eq!(artifact.kind, ArtifactKind::DownloadedVideo);

        match artifact.location {
            domain::media::ArtifactLocation::LocalPath(path_str) => {
                let path = PathBuf::from(path_str);
                assert!(path.exists());
                // Cleanup
                fs::remove_file(path).await.unwrap();
            }
            _ => panic!("Expected LocalPath"),
        }

        fs::remove_dir_all(target_dir).await.ok();
    }
}
