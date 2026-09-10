use async_trait::async_trait;
use domain::media::{Artifact, MediaSource, SubtitleTrack};
use ports::error::PortError;
use ports::source::{DownloadSubtitleRequest, SubtitleSourcePort};
use std::sync::atomic::{AtomicBool, Ordering};

static PAUSE_REACHED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Default)]
pub struct NativeE2ePausedSubtitleSource;

impl NativeE2ePausedSubtitleSource {
    pub fn reset() {
        PAUSE_REACHED.store(false, Ordering::Release);
    }

    pub fn pause_reached() -> bool {
        PAUSE_REACHED.load(Ordering::Acquire)
    }
}

#[async_trait]
impl SubtitleSourcePort for NativeE2ePausedSubtitleSource {
    async fn list_subtitles(&self, _source: &MediaSource) -> Result<Vec<SubtitleTrack>, PortError> {
        Ok(vec![SubtitleTrack {
            id: "native-e2e-vtt".to_string(),
            language: "en".to_string(),
            label: Some("Native E2E".to_string()),
            format: Some("vtt".to_string()),
            is_auto_generated: false,
        }])
    }

    async fn download_subtitle(
        &self,
        _request: DownloadSubtitleRequest,
    ) -> Result<Artifact, PortError> {
        PAUSE_REACHED.store(true, Ordering::Release);
        std::future::pending().await
    }
}
