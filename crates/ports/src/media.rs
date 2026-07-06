use async_trait::async_trait;

use crate::error::PortError;
use domain::media::{Artifact, MediaMetadata};

use std::path::Path;

#[async_trait]
pub trait MediaProbePort: Send + Sync {
    async fn probe_local_file(&self, path: &Path) -> Result<MediaMetadata, PortError>;
}

#[async_trait]
pub trait MediaMuxerPort: Send + Sync {
    async fn extract_audio(
        &self,
        video_artifact: &Artifact,
        output_path: &std::path::Path,
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
