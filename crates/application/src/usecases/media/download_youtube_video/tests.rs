use super::test_support::{FailingVideoSourceAdapter, MockArtifactStore};
use super::usecase::{DownloadYoutubeVideoRequest, DownloadYoutubeVideoUseCase};

use adapters_storage::memory::InMemoryProjectRepository;
use adapters_ytdlp::mock::MockVideoSourceAdapter;
use domain::media::MediaSource;
use domain::project::{Project, ProjectId};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::error::ApplicationError;
use crate::test_utils::MockStorageUnitOfWork;
use ports::repository::ProjectRepository;

#[tokio::test]
async fn test_download_video_success() {
    let repo = InMemoryProjectRepository::new(std::sync::Arc::new(std::sync::Mutex::new(
        adapters_storage::memory::InMemoryDatabase::new(),
    )));
    let source_port = MockVideoSourceAdapter::new();
    let transaction_gateway = MockStorageUnitOfWork::new();
    let artifact_store = MockArtifactStore {
        fail_on_stage: Arc::new(AtomicBool::new(false)),
        deleted_keys: Arc::new(std::sync::Mutex::new(vec![])),
    };

    let mut project = Project::new("Test Project".to_string());
    project
        .import_source(
            MediaSource::YoutubeUrl {
                url: "https://youtube.com/watch?v=123".to_string(),
            },
            None,
        )
        .unwrap();
    let project_id = project.id().clone();
    repo.create(project).await.unwrap();

    let use_case =
        DownloadYoutubeVideoUseCase::new(repo, source_port, artifact_store, transaction_gateway);

    let _temp_guard = tempfile::tempdir().unwrap();
    let temp_dir = _temp_guard.path().to_path_buf();
    let request = DownloadYoutubeVideoRequest {
        project_id,
        temp_dir: temp_dir.clone(),
        filename_hint: Some("video.mp4".into()),
    };

    let result = use_case.execute(request).await;
    assert!(result.is_ok(), "Expected success: {:?}", result.err());
}

#[tokio::test]
async fn test_download_missing_project_fails_before_download() {
    let repo = InMemoryProjectRepository::new(std::sync::Arc::new(std::sync::Mutex::new(
        adapters_storage::memory::InMemoryDatabase::new(),
    )));
    // project is missing

    let use_case = DownloadYoutubeVideoUseCase::new(
        repo,
        MockVideoSourceAdapter::new(),
        MockArtifactStore {
            fail_on_stage: Arc::new(AtomicBool::new(false)),
            deleted_keys: Arc::new(std::sync::Mutex::new(vec![])),
        },
        MockStorageUnitOfWork::new(),
    );

    let _temp_guard = tempfile::tempdir().unwrap();
    let temp_dir = _temp_guard.path().to_path_buf();
    let request = DownloadYoutubeVideoRequest {
        project_id: ProjectId::new(),
        temp_dir,
        filename_hint: None,
    };

    let err = use_case.execute(request).await.unwrap_err();
    assert!(matches!(err, ApplicationError::ProjectNotFound(_)));
}

#[tokio::test]
async fn test_non_youtube_source_fails() {
    let repo = InMemoryProjectRepository::new(std::sync::Arc::new(std::sync::Mutex::new(
        adapters_storage::memory::InMemoryDatabase::new(),
    )));
    let mut project = Project::new("Test Project".to_string());
    project
        .import_source(
            MediaSource::ExternalLocalFile {
                path: "".to_string(),
            },
            None,
        )
        .unwrap();
    let project_id = project.id().clone();
    repo.create(project).await.unwrap();

    let use_case = DownloadYoutubeVideoUseCase::new(
        repo,
        MockVideoSourceAdapter::new(),
        MockArtifactStore {
            fail_on_stage: Arc::new(AtomicBool::new(false)),
            deleted_keys: Arc::new(std::sync::Mutex::new(vec![])),
        },
        MockStorageUnitOfWork::new(),
    );

    let _temp_guard = tempfile::tempdir().unwrap();
    let temp_dir = _temp_guard.path().to_path_buf();
    let request = DownloadYoutubeVideoRequest {
        project_id,
        temp_dir,
        filename_hint: None,
    };

    let err = use_case.execute(request).await.unwrap_err();
    assert!(matches!(err, ApplicationError::InvalidOperation { .. }));
}

#[tokio::test]
async fn test_wrong_location_from_port_fails() {
    let repo = InMemoryProjectRepository::new(std::sync::Arc::new(std::sync::Mutex::new(
        adapters_storage::memory::InMemoryDatabase::new(),
    )));
    let mut project = Project::new("Test Project".to_string());
    project
        .import_source(
            MediaSource::YoutubeUrl {
                url: "https://youtube.com/watch?v=123".to_string(),
            },
            None,
        )
        .unwrap();
    let project_id = project.id().clone();
    repo.create(project).await.unwrap();

    let use_case = DownloadYoutubeVideoUseCase::new(
        repo,
        FailingVideoSourceAdapter,
        MockArtifactStore {
            fail_on_stage: Arc::new(AtomicBool::new(false)),
            deleted_keys: Arc::new(std::sync::Mutex::new(vec![])),
        },
        MockStorageUnitOfWork::new(),
    );

    let _temp_guard = tempfile::tempdir().unwrap();
    let temp_dir = _temp_guard.path().to_path_buf();
    let request = DownloadYoutubeVideoRequest {
        project_id,
        temp_dir,
        filename_hint: None,
    };

    let err = use_case.execute(request).await.unwrap_err();
    assert!(matches!(err, ApplicationError::InvalidOperation { .. }));
}

#[tokio::test]
async fn test_import_failure_propagates() {
    let repo = InMemoryProjectRepository::new(std::sync::Arc::new(std::sync::Mutex::new(
        adapters_storage::memory::InMemoryDatabase::new(),
    )));
    let mut project = Project::new("Test Project".to_string());
    project
        .import_source(
            MediaSource::YoutubeUrl {
                url: "https://youtube.com/watch?v=123".to_string(),
            },
            None,
        )
        .unwrap();
    let project_id = project.id().clone();
    repo.create(project).await.unwrap();

    let use_case = DownloadYoutubeVideoUseCase::new(
        repo,
        MockVideoSourceAdapter::new(),
        MockArtifactStore {
            fail_on_stage: Arc::new(AtomicBool::new(true)),
            deleted_keys: Arc::new(std::sync::Mutex::new(vec![])),
        },
        MockStorageUnitOfWork::new(),
    );

    let _temp_guard = tempfile::tempdir().unwrap();
    let temp_dir = _temp_guard.path().to_path_buf();
    let request = DownloadYoutubeVideoRequest {
        project_id,
        temp_dir,
        filename_hint: None,
    };

    let err = use_case.execute(request).await.unwrap_err();
    assert!(matches!(err, ApplicationError::Port(_)));
}

#[tokio::test]
async fn test_transaction_failure_deletes_staged_artifact() {
    let repo = InMemoryProjectRepository::new(std::sync::Arc::new(std::sync::Mutex::new(
        adapters_storage::memory::InMemoryDatabase::new(),
    )));
    let mut project = Project::new("Test Project".to_string());
    project
        .import_source(
            MediaSource::YoutubeUrl {
                url: "https://youtube.com/watch?v=123".to_string(),
            },
            None,
        )
        .unwrap();
    let project_id = project.id().clone();
    repo.create(project).await.unwrap();

    let deleted_keys = Arc::new(std::sync::Mutex::new(vec![]));
    let artifact_store = MockArtifactStore {
        fail_on_stage: Arc::new(AtomicBool::new(false)),
        deleted_keys: deleted_keys.clone(),
    };

    let use_case = DownloadYoutubeVideoUseCase::new(
        repo,
        MockVideoSourceAdapter::new(),
        artifact_store,
        MockStorageUnitOfWork::with_failure(),
    );

    let _temp_guard = tempfile::tempdir().unwrap();
    let temp_dir = _temp_guard.path().to_path_buf();
    let request = DownloadYoutubeVideoRequest {
        project_id,
        temp_dir,
        filename_hint: None,
    };

    let err = use_case.execute(request).await.unwrap_err();
    assert!(matches!(err, ApplicationError::Port(_)));

    let deleted = deleted_keys.lock().unwrap();
    assert_eq!(
        deleted.len(),
        1,
        "Should have deleted the staged artifact key"
    );
    assert!(deleted[0].starts_with("staging."));
}
