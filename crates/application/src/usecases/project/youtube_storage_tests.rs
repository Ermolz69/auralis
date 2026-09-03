use std::sync::{Arc, Mutex};

use adapters_storage::local::{LocalArtifactStore, LocalTempWorkspace};
use adapters_storage::sqlite::{
    SqliteArtifactIndex, SqliteOutboxRepository, SqliteProjectRepository, SqliteStorageUnitOfWork,
    connect_sqlite,
};
use domain::media::{ArtifactKind, ArtifactState};
use domain::outbox::OutboxPayload;
use ports::artifact_index::ArtifactIndex;
use ports::repository::OutboxRepository;
use ports::storage::ArtifactStore;
use ports::workspace::TempWorkspacePort;

use super::create_from_youtube::{
    CreateProjectFromYoutubeRequest, CreateProjectFromYoutubeUseCase,
};
use super::youtube_storage_support::{FileVideoSource, UnusedEventPublisher};
use crate::worker::outbox::{
    OutboxWorker, maintenance::OutboxMaintenanceConfig, payload_handler::PayloadHandler,
};

async fn assert_download_cleanup(extension: &'static str) {
    let root = tempfile::tempdir().unwrap();
    let pool = connect_sqlite(root.path().join("test.db")).await.unwrap();
    let project_repo = Arc::new(SqliteProjectRepository::new(pool.clone()));
    let artifact_store = Arc::new(LocalArtifactStore::new(root.path().join("artifacts")));
    let artifact_index = Arc::new(SqliteArtifactIndex::new(pool.clone()));
    let outbox_repo = SqliteOutboxRepository::new(pool.clone());
    let uow = Arc::new(SqliteStorageUnitOfWork::new(pool.clone()));
    let workspace = Arc::new(LocalTempWorkspace::new(root.path().to_path_buf()));
    let downloaded_paths = Arc::new(Mutex::new(Vec::new()));

    let response = CreateProjectFromYoutubeUseCase::new(
        project_repo,
        FileVideoSource {
            extension,
            downloaded_paths: downloaded_paths.clone(),
        },
        artifact_store.clone(),
        uow.clone(),
        workspace.clone(),
        Arc::new(super::lifecycle::ProjectLifecycleLocks::new()),
    )
    .execute(CreateProjectFromYoutubeRequest {
        url: "https://www.youtube.com/watch?v=cleanup-test".into(),
        project_id: None,
    })
    .await
    .unwrap();

    let downloaded_path = downloaded_paths.lock().unwrap()[0].clone();
    assert_eq!(
        downloaded_path.file_name().unwrap(),
        format!("original.{extension}").as_str()
    );
    assert!(!downloaded_path.exists());
    let allocation_dir = downloaded_path.parent().unwrap();
    assert!(allocation_dir.is_dir());
    assert_eq!(std::fs::read_dir(allocation_dir).unwrap().count(), 0);

    let sibling = workspace
        .create_allocation(response.project.id(), "keep-other-download")
        .await
        .unwrap();
    let sibling_file = sibling.absolute_path.join("keep.txt");
    tokio::fs::write(&sibling_file, b"keep").await.unwrap();

    let pending = outbox_repo.fetch_pending(10).await.unwrap();
    assert_eq!(pending.messages.len(), 2);
    assert_eq!(pending.corrupted_isolated, 0);
    let cleanup = pending
        .messages
        .iter()
        .find(|message| {
            matches!(
                message.payload,
                OutboxPayload::DeleteWorkspaceAllocation { .. }
            )
        })
        .unwrap();
    let OutboxPayload::DeleteWorkspaceAllocation { workspace_key } = &cleanup.payload else {
        unreachable!()
    };
    assert!(workspace_key.as_str().starts_with(&format!(
        "tmp/{}/youtube-video-download_",
        response.project.id()
    )));
    assert_eq!(
        workspace.resolve_key(workspace_key).await.unwrap(),
        allocation_dir
    );
    assert!(
        artifact_index
            .list_by_project(response.project.id())
            .await
            .unwrap()
            .is_empty()
    );

    pool.close().await;
    let pool = connect_sqlite(root.path().join("test.db")).await.unwrap();
    let outbox_repo = SqliteOutboxRepository::new(pool.clone());
    let artifact_index = Arc::new(SqliteArtifactIndex::new(pool.clone()));
    let uow = Arc::new(SqliteStorageUnitOfWork::new(pool.clone()));
    let worker = OutboxWorker::new(
        outbox_repo.clone(),
        artifact_store.clone(),
        artifact_index.clone(),
        uow.clone(),
        Arc::new(UnusedEventPublisher),
        workspace.clone(),
        OutboxMaintenanceConfig::try_default().unwrap(),
    );
    let report = worker.process_pending_messages(10).await.unwrap();
    assert_eq!(report.fetched, 2);
    assert_eq!(report.completed, 2);
    assert_eq!(report.retry_scheduled, 0);
    assert_eq!(report.dead, 0);
    assert_eq!(report.storage_errors, 0);
    assert_eq!(report.corrupted, 0);
    assert!(!allocation_dir.exists());
    assert_eq!(tokio::fs::read(&sibling_file).await.unwrap(), b"keep");

    let artifacts = artifact_index
        .list_by_project(response.project.id())
        .await
        .unwrap();
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].kind, ArtifactKind::DownloadedVideo);
    assert_eq!(artifacts[0].state, ArtifactState::Ready);
    let final_path = artifact_store
        .resolve_artifact(&artifacts[0])
        .await
        .unwrap();
    assert_eq!(final_path.extension().unwrap(), extension);
    assert_eq!(
        tokio::fs::read(&final_path).await.unwrap(),
        b"downloaded video"
    );

    let replay = PayloadHandler {
        artifact_store,
        artifact_index,
        uow,
        event_publisher: Arc::new(UnusedEventPublisher),
        workspace_port: workspace,
    };
    replay
        .process_payload(&cleanup.id, &cleanup.payload)
        .await
        .unwrap();
    assert!(!allocation_dir.exists());
    assert_eq!(tokio::fs::read(&sibling_file).await.unwrap(), b"keep");
    assert!(
        outbox_repo
            .fetch_pending(10)
            .await
            .unwrap()
            .messages
            .is_empty()
    );
    assert_eq!(
        worker.process_pending_messages(10).await.unwrap().fetched,
        0
    );
    pool.close().await;
}

#[tokio::test]
async fn mp4_download_finalizes_and_cleans_its_workspace_allocation() {
    assert_download_cleanup("mp4").await;
}

#[tokio::test]
async fn webm_download_finalizes_and_cleans_its_workspace_allocation() {
    assert_download_cleanup("webm").await;
}
