use async_trait::async_trait;

use domain::media::{Artifact, MediaMetadata};
use crate::error::PortError;

#[async_trait]
pub trait MediaMuxerPort: Send + Sync {
    async fn probe_media(&self, media_artifact: &Artifact) -> Result<MediaMetadata, PortError>;
    
    async fn extract_audio(
        &self, 
        video_artifact: &Artifact, 
        output_path: &std::path::Path
    ) -> Result<Artifact, PortError>;

    async fn mux_audio(
        &self,
        video_artifact: &Artifact,
        audio_artifact: &Artifact,
        output_path: &std::path::Path,
    ) -> Result<Artifact, PortError>;

    async fn render_preview(
        &self,
        video_artifact: &Artifact,
        audio_artifact: &Artifact,
        output_path: &std::path::Path,
    ) -> Result<Artifact, PortError>;
}
