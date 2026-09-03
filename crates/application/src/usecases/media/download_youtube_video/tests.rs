#![allow(clippy::unwrap_used)]

use super::test_support::{FailingVideoSourceAdapter, MockArtifactStore};
use super::usecase::{DownloadYoutubeVideoRequest, DownloadYoutubeVideoUseCase};
use crate::error::ApplicationError;
use adapters_ytdlp::mock::MockVideoSourceAdapter;
use domain::media::MediaSource;
use domain::outbox::WorkspaceKey;
use domain::project::ProjectId;
use std::sync::{Arc, atomic::AtomicBool};

fn store(fail: bool) -> MockArtifactStore {
    MockArtifactStore {
        fail_on_stage: Arc::new(AtomicBool::new(fail)),
        deleted_keys: Arc::default(),
    }
}

fn request(root: &std::path::Path) -> DownloadYoutubeVideoRequest {
    let project_id = ProjectId::new();
    DownloadYoutubeVideoRequest {
        workspace_key: WorkspaceKey::new(format!("tmp/{project_id}/download")).unwrap(),
        project_id,
        source: MediaSource::YoutubeUrl {
            url: "https://youtube.com/watch?v=test".into(),
        },
        temp_dir: root.to_path_buf(),
        filename_hint: Some("video.mp4".into()),
    }
}

#[tokio::test]
async fn download_only_prepares_a_write_and_preserves_the_allocation_key() {
    let root = tempfile::tempdir().unwrap();
    let request = request(root.path());
    let key = request.workspace_key.clone();
    let write = DownloadYoutubeVideoUseCase::new(MockVideoSourceAdapter::new(), store(false))
        .execute(request)
        .await
        .unwrap();
    assert_eq!(write.temp_workspace_key, Some(key));
    assert_eq!(
        write.artifact.state,
        domain::media::ArtifactState::PendingFinalize
    );
}

#[tokio::test]
async fn non_youtube_source_fails() {
    let root = tempfile::tempdir().unwrap();
    let mut request = request(root.path());
    request.source = MediaSource::ExternalLocalFile {
        path: "video.mp4".into(),
    };
    let result = DownloadYoutubeVideoUseCase::new(MockVideoSourceAdapter::new(), store(false))
        .execute(request)
        .await;
    assert!(matches!(
        result,
        Err(ApplicationError::InvalidOperation { .. })
    ));
}

#[tokio::test]
async fn wrong_location_from_port_fails() {
    let root = tempfile::tempdir().unwrap();
    let result = DownloadYoutubeVideoUseCase::new(FailingVideoSourceAdapter, store(false))
        .execute(request(root.path()))
        .await;
    assert!(matches!(
        result,
        Err(ApplicationError::InvalidOperation { .. })
    ));
}

#[tokio::test]
async fn staging_failure_propagates() {
    let root = tempfile::tempdir().unwrap();
    let result = DownloadYoutubeVideoUseCase::new(MockVideoSourceAdapter::new(), store(true))
        .execute(request(root.path()))
        .await;
    assert!(matches!(result, Err(ApplicationError::Port(_))));
}
