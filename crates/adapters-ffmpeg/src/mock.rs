use async_trait::async_trait;
use std::path::Path;

use domain::media::{Artifact, ArtifactKind, MediaMetadata};
use ports::error::PortError;
use ports::media::{MediaMuxerPort, MediaProbePort};

#[derive(Default, Clone)]
pub struct MockMediaProbeAdapter;

impl MockMediaProbeAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MediaProbePort for MockMediaProbeAdapter {
    async fn probe_local_file(&self, _path: &Path) -> Result<MediaMetadata, PortError> {
        Ok(MediaMetadata {
            duration_ms: 5000,
            width: Some(1920),
            height: Some(1080),
            fps: Some(60.0),
            video_codec: Some("mock_video_codec".to_string()),
            audio_codec: Some("mock_audio_codec".to_string()),
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
}

pub struct MockMediaMuxerAdapter;

impl Default for MockMediaMuxerAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl MockMediaMuxerAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MediaMuxerPort for MockMediaMuxerAdapter {
    async fn extract_audio(
        &self,
        _video_artifact: &Artifact,
        output_path: &Path,
    ) -> Result<Artifact, PortError> {
        Ok(Artifact {
            id: domain::media::ArtifactId::new(),
            kind: ArtifactKind::ExtractedAudio,
            location: domain::media::ArtifactLocation::LocalPath(
                output_path.to_string_lossy().to_string(),
            ),
            size_bytes: None,
            state: domain::media::ArtifactState::Ready,
            created_at: domain::chrono::Utc::now(),
            updated_at: domain::chrono::Utc::now(),
            ready_at: Some(domain::chrono::Utc::now()),
        })
    }

    async fn mux_audio(
        &self,
        _video_artifact: &Artifact,
        _audio_artifact: &Artifact,
        output_path: &Path,
    ) -> Result<Artifact, PortError> {
        Ok(Artifact {
            id: domain::media::ArtifactId::new(),
            kind: ArtifactKind::FinalVideo,
            location: domain::media::ArtifactLocation::LocalPath(
                output_path.to_string_lossy().to_string(),
            ),
            size_bytes: None,
            state: domain::media::ArtifactState::Ready,
            created_at: domain::chrono::Utc::now(),
            updated_at: domain::chrono::Utc::now(),
            ready_at: Some(domain::chrono::Utc::now()),
        })
    }

    async fn render_preview(
        &self,
        _video_artifact: &Artifact,
        _audio_artifact: &Artifact,
        output_path: &Path,
    ) -> Result<Artifact, PortError> {
        Ok(Artifact {
            id: domain::media::ArtifactId::new(),
            kind: ArtifactKind::PreviewVideo,
            location: domain::media::ArtifactLocation::LocalPath(
                output_path.to_string_lossy().to_string(),
            ),
            size_bytes: None,
            state: domain::media::ArtifactState::Ready,
            created_at: domain::chrono::Utc::now(),
            updated_at: domain::chrono::Utc::now(),
            ready_at: Some(domain::chrono::Utc::now()),
        })
    }
}
