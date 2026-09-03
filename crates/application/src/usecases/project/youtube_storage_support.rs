use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use adapters_ytdlp::mock::MockVideoSourceAdapter;
use async_trait::async_trait;
use domain::media::{Artifact, MediaMetadata, MediaSource};
use ports::error::PortError;
use ports::events::AppEventPublisher;
use ports::source::{DownloadMediaRequest, VideoSourcePort};

#[derive(Clone)]
pub(super) struct FileVideoSource {
    pub extension: &'static str,
    pub downloaded_paths: Arc<Mutex<Vec<PathBuf>>>,
}

#[async_trait]
impl VideoSourcePort for FileVideoSource {
    async fn validate_source(&self, source: &MediaSource) -> Result<(), PortError> {
        MockVideoSourceAdapter::new().validate_source(source).await
    }

    async fn fetch_metadata(&self, source: &MediaSource) -> Result<MediaMetadata, PortError> {
        MockVideoSourceAdapter::new().fetch_metadata(source).await
    }

    async fn download_media(
        &self,
        mut request: DownloadMediaRequest,
    ) -> Result<Artifact, PortError> {
        let filename = format!(
            "{}.{}",
            request.filename_hint.as_deref().unwrap_or("download"),
            self.extension
        );
        let path = request.target_dir.join(&filename);
        tokio::fs::write(&path, b"downloaded video").await.unwrap();
        self.downloaded_paths.lock().unwrap().push(path);
        request.filename_hint = Some(filename);
        MockVideoSourceAdapter::new().download_media(request).await
    }
}

pub(super) struct UnusedEventPublisher;

#[async_trait]
impl AppEventPublisher for UnusedEventPublisher {
    async fn publish_project_updated(&self, _project_id: &str) -> Result<(), PortError> {
        panic!("Artifact cleanup must not publish project lifecycle events")
    }

    async fn publish_transcript_ready(
        &self,
        _project_id: &str,
        _job_id: &str,
    ) -> Result<(), PortError> {
        panic!("Video download must not publish transcript events")
    }
}
