use std::path::Path;
use async_trait::async_trait;

use domain::media::{Artifact, ArtifactKind, MediaMetadata};
use ports::error::PortError;
use ports::media::MediaMuxerPort;

pub struct MockMediaMuxerAdapter;

impl MockMediaMuxerAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MediaMuxerPort for MockMediaMuxerAdapter {
    async fn probe_media(&self, _media_artifact: &Artifact) -> Result<MediaMetadata, PortError> {
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
            has_video: true,
            has_audio: true,
        })
    }
    
    async fn extract_audio(
        &self, 
        _video_artifact: &Artifact, 
        output_path: &Path
    ) -> Result<Artifact, PortError> {
        Ok(Artifact {
            id: domain::media::ArtifactId(uuid::Uuid::new_v4()),
            kind: ArtifactKind::ExtractedAudio,
            location: domain::media::ArtifactLocation::LocalPath(output_path.to_string_lossy().to_string()),
        })
    }

    async fn mux_audio(
        &self,
        _video_artifact: &Artifact,
        _audio_artifact: &Artifact,
        output_path: &Path,
    ) -> Result<Artifact, PortError> {
        Ok(Artifact {
            id: domain::media::ArtifactId(uuid::Uuid::new_v4()),
            kind: ArtifactKind::FinalVideo,
            location: domain::media::ArtifactLocation::LocalPath(output_path.to_string_lossy().to_string()),
        })
    }

    async fn render_preview(
        &self,
        _video_artifact: &Artifact,
        _audio_artifact: &Artifact,
        output_path: &Path,
    ) -> Result<Artifact, PortError> {
        Ok(Artifact {
            id: domain::media::ArtifactId(uuid::Uuid::new_v4()),
            kind: ArtifactKind::PreviewVideo,
            location: domain::media::ArtifactLocation::LocalPath(output_path.to_string_lossy().to_string()),
        })
    }
}
