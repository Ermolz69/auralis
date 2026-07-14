use super::store::LocalArtifactStore;
use domain::media::ArtifactKind;
use domain::project::ProjectId;
use ports::error::PortError;
use ports::storage::ArtifactStore;
use tempfile::tempdir;

#[tokio::test]
async fn test_resolve_artifact() {
    let temp_dir = tempdir().unwrap();
    let store = LocalArtifactStore::new(temp_dir.path().to_path_buf());
    let _project_id = ProjectId(uuid::Uuid::new_v4());

    let artifact = domain::media::Artifact {
        id: domain::media::ArtifactId::new(),
        kind: ArtifactKind::LogFile,
        location: domain::media::ArtifactLocation::StorageKey("test.log".to_string()),
        size_bytes: None,
        state: domain::media::ArtifactState::Ready,
        created_at: domain::chrono::Utc::now(),
        updated_at: domain::chrono::Utc::now(),
        ready_at: Some(domain::chrono::Utc::now()),
    };

    let resolved_path = store.resolve_artifact(&artifact).await.unwrap();
    assert!(resolved_path.starts_with(temp_dir.path()));

    // Also check legacy LocalPath

    let legacy_path = store.resolve_legacy_local_path("/tmp/legacy.log").unwrap();
    assert_eq!(legacy_path, std::path::PathBuf::from("/tmp/legacy.log"));
}

#[tokio::test]
async fn test_stage_owned_temp_file_creates_pending_artifact() {
    let temp_dir = tempdir().unwrap();
    let store = LocalArtifactStore::new(temp_dir.path().to_path_buf());
    let project_id = ProjectId(uuid::Uuid::new_v4());

    let source_dir = tempdir().unwrap();
    let source_path = source_dir.path().join("video.mp4");
    tokio::fs::write(&source_path, b"video data").await.unwrap();

    let staged = store
        .stage_owned_temp_file(&project_id, ArtifactKind::SourceVideo, &source_path, None)
        .await
        .unwrap();

    assert_eq!(
        staged.artifact.state,
        domain::media::ArtifactState::PendingFinalize
    );
    assert_eq!(staged.size_bytes, 10);
    assert!(staged.staging_key.starts_with(".staging/"));
    assert!(!tokio::fs::try_exists(&source_path).await.unwrap());
    let staging_path = store.resolve_storage_key(&staged.staging_key).unwrap();
    assert!(tokio::fs::try_exists(&staging_path).await.unwrap());
}

#[tokio::test]
async fn test_finalize_moves_to_final() {
    let temp_dir = tempdir().unwrap();
    let store = LocalArtifactStore::new(temp_dir.path().to_path_buf());
    let project_id = ProjectId(uuid::Uuid::new_v4());

    let source_dir = tempdir().unwrap();
    let source_path = source_dir.path().join("video.mp4");
    tokio::fs::write(&source_path, b"video data").await.unwrap();

    let staged = store
        .import_external_file(&project_id, ArtifactKind::SourceVideo, &source_path, None)
        .await
        .unwrap();

    store
        .finalize_staged_artifact(&staged.staging_key, &staged.final_key)
        .await
        .unwrap();

    let staging_path = store.resolve_storage_key(&staged.staging_key).unwrap();
    let final_path = store.resolve_storage_key(&staged.final_key).unwrap();

    assert!(!tokio::fs::try_exists(&staging_path).await.unwrap());
    assert!(tokio::fs::try_exists(&final_path).await.unwrap());
}

#[tokio::test]
async fn test_finalize_is_idempotent_when_final_exists() {
    let temp_dir = tempdir().unwrap();
    let store = LocalArtifactStore::new(temp_dir.path().to_path_buf());
    let project_id = ProjectId(uuid::Uuid::new_v4());

    let source_dir = tempdir().unwrap();
    let source_path = source_dir.path().join("video.mp4");
    tokio::fs::write(&source_path, b"video data").await.unwrap();

    let staged = store
        .import_external_file(&project_id, ArtifactKind::SourceVideo, &source_path, None)
        .await
        .unwrap();

    store
        .finalize_staged_artifact(&staged.staging_key, &staged.final_key)
        .await
        .unwrap();

    // Finalize again should be ok
    let result = store
        .finalize_staged_artifact(&staged.staging_key, &staged.final_key)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_finalize_fails_when_both_missing() {
    let temp_dir = tempdir().unwrap();
    let store = LocalArtifactStore::new(temp_dir.path().to_path_buf());

    let result = store
        .finalize_staged_artifact(".staging/missing", "missing_final.txt")
        .await;
    assert!(result.is_err());
    if let Err(PortError::Io { message }) = result {
        assert!(message.contains("missing"));
    } else {
        panic!("Expected Io error");
    }
}

#[tokio::test]
async fn test_delete_storage_key_is_idempotent() {
    let temp_dir = tempdir().unwrap();
    let store = LocalArtifactStore::new(temp_dir.path().to_path_buf());

    let result = store.delete_storage_key("some_missing_key.txt").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_storage_key_rejects_absolute_path() {
    let temp_dir = tempdir().unwrap();
    let store = LocalArtifactStore::new(temp_dir.path().to_path_buf());

    #[cfg(target_os = "windows")]
    let key = "C:\\Windows\\System32\\cmd.exe";
    #[cfg(not(target_os = "windows"))]
    let key = "/etc/passwd";

    let result = store.resolve_storage_key(key);
    assert!(result.is_err());
    if let Err(PortError::Unexpected { message }) = result {
        assert!(message.contains("clean relative path"));
    } else {
        panic!("Expected Unexpected error");
    }
}

#[tokio::test]
async fn test_storage_key_rejects_parent_dir() {
    let temp_dir = tempdir().unwrap();
    let store = LocalArtifactStore::new(temp_dir.path().to_path_buf());

    let result = store.resolve_storage_key("some/../../path.txt");
    assert!(result.is_err());
    if let Err(PortError::Unexpected { message }) = result {
        assert!(message.contains("clean relative path"));
    } else {
        panic!("Expected Unexpected error");
    }
}
