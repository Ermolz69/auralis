use async_trait::async_trait;
use std::path::{Path, PathBuf};

use domain::media::{Artifact, MediaMetadata, MediaSource, SubtitleTrack};

use crate::error::PortError;

pub struct DownloadMediaRequest {
    pub source: MediaSource,
    pub target_dir: PathBuf,
    pub filename_hint: Option<String>,
}

#[async_trait]
pub trait VideoSourcePort: Send + Sync {
    async fn validate_source(&self, source: &MediaSource) -> Result<(), PortError>;
    async fn fetch_metadata(&self, source: &MediaSource) -> Result<MediaMetadata, PortError>;
    async fn download_media(&self, request: DownloadMediaRequest) -> Result<Artifact, PortError>;
}

#[async_trait]
pub trait SubtitleSourcePort: Send + Sync {
    async fn list_subtitles(&self, source: &MediaSource) -> Result<Vec<SubtitleTrack>, PortError>;
    async fn download_subtitle(
        &self,
        source: &MediaSource,
        track: &SubtitleTrack,
        target_path: &Path,
    ) -> Result<Artifact, PortError>;
}
