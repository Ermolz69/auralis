use async_trait::async_trait;
use std::path::Path;

use domain::media::{Artifact, ArtifactKind, MediaMetadata, MediaSource, SubtitleTrack};
use ports::error::PortError;
use ports::source::{DownloadMediaRequest, SubtitleSourcePort, VideoSourcePort};

pub struct MockVideoSourceAdapter {
    pub should_fail_validation: bool,
}

impl Default for MockVideoSourceAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MockVideoSourceAdapter {
    pub fn new() -> Self {
        Self {
            should_fail_validation: false,
        }
    }

    pub fn failing() -> Self {
        Self {
            should_fail_validation: true,
        }
    }
}

#[async_trait]
impl VideoSourcePort for MockVideoSourceAdapter {
    async fn validate_source(&self, _source: &MediaSource) -> Result<(), PortError> {
        if self.should_fail_validation {
            return Err(PortError::InvalidSource {
                message: "Validation failed".to_string(),
            });
        }
        Ok(())
    }

    async fn fetch_metadata(&self, _source: &MediaSource) -> Result<MediaMetadata, PortError> {
        Ok(MediaMetadata {
            duration_ms: 1000,
            width: Some(1920),
            height: Some(1080),
            fps: Some(60.0),
            video_codec: Some("h264".to_string()),
            audio_codec: Some("aac".to_string()),
            audio_channels: Some(2),
            sample_rate: Some(48000),
            container: Some("mp4".to_string()),
            bitrate: Some(5000000),
            format_name: Some("mov,mp4,m4a,3gp,3g2,mj2".to_string()),
            has_video: true,
            has_audio: true,
            streams: vec![],
            video: None,
            audio_tracks: vec![],
        })
    }

    async fn download_media(&self, request: DownloadMediaRequest) -> Result<Artifact, PortError> {
        // Just return a mock local file artifact
        let path = request.target_dir.join(
            request
                .filename_hint
                .unwrap_or_else(|| "mock_video.mp4".to_string()),
        );
        Ok(Artifact {
            id: domain::media::ArtifactId::new(),
            kind: ArtifactKind::SourceVideo,
            location: domain::media::ArtifactLocation::LocalPath(
                path.to_string_lossy().to_string(),
            ),
            size_bytes: None,
        })
    }
}

pub struct MockSubtitleSourceAdapter;

impl Default for MockSubtitleSourceAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MockSubtitleSourceAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SubtitleSourcePort for MockSubtitleSourceAdapter {
    async fn list_subtitles(&self, _source: &MediaSource) -> Result<Vec<SubtitleTrack>, PortError> {
        Ok(vec![SubtitleTrack {
            id: "en".to_string(),
            language: "en".to_string(),
            label: Some("English".to_string()),
            format: Some("vtt".to_string()),
            is_auto_generated: false,
        }])
    }

    async fn download_subtitle(
        &self,
        _source: &MediaSource,
        track: &SubtitleTrack,
        target_path: &Path,
    ) -> Result<Artifact, PortError> {
        let path = target_path.join(format!("mock_sub_{}.vtt", track.language));
        Ok(Artifact {
            id: domain::media::ArtifactId::new(),
            kind: ArtifactKind::OriginalSubtitle,
            location: domain::media::ArtifactLocation::LocalPath(
                path.to_string_lossy().to_string(),
            ),
            size_bytes: None,
        })
    }
}
