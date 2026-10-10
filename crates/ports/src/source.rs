use async_trait::async_trait;
use std::path::PathBuf;

use domain::media::{Artifact, MediaMetadata, MediaSource, SubtitleTrack};

use crate::error::PortError;

pub struct DownloadMediaRequest {
    pub source: MediaSource,
    pub target_dir: PathBuf,
    pub filename_hint: Option<String>,
}

pub struct DownloadSubtitleRequest {
    pub source: MediaSource,
    pub track: SubtitleTrack,
    pub target_directory: PathBuf,
}

#[async_trait]
pub trait VideoSourcePort: Send + Sync {
    async fn validate_source(&self, source: &MediaSource) -> Result<(), PortError>;
    async fn fetch_metadata(&self, source: &MediaSource) -> Result<MediaMetadata, PortError>;
    async fn download_media(&self, request: DownloadMediaRequest) -> Result<Artifact, PortError>;
}

#[async_trait]
pub trait SubtitleSourcePort: Send + Sync {
    /// The compatibility default drains the operation; process adapters override to stop and join.
    async fn list_subtitles_cancellable(
        &self,
        source: &MediaSource,
        token: &crate::cancellation::CancellationToken,
    ) -> Result<Vec<SubtitleTrack>, PortError> {
        if token.is_cancelled() {
            return Err(PortError::Cancelled);
        }
        let result = self.list_subtitles(source).await;
        if token.is_cancelled() {
            Err(PortError::Cancelled)
        } else {
            result
        }
    }

    async fn download_subtitle_cancellable(
        &self,
        request: DownloadSubtitleRequest,
        token: &crate::cancellation::CancellationToken,
    ) -> Result<Artifact, PortError> {
        if token.is_cancelled() {
            return Err(PortError::Cancelled);
        }
        let result = self.download_subtitle(request).await;
        if token.is_cancelled() {
            Err(PortError::Cancelled)
        } else {
            result
        }
    }

    async fn list_subtitles(&self, source: &MediaSource) -> Result<Vec<SubtitleTrack>, PortError>;
    async fn download_subtitle(
        &self,
        request: DownloadSubtitleRequest,
    ) -> Result<Artifact, PortError>;
}
