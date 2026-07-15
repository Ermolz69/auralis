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

use crate::local::artifact_store::staging::{FileOps, stage_owned_temp_file_with_ops};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
struct FakeFileOps {
    pub expected_source_path: std::path::PathBuf,
    pub rename_result: Arc<Mutex<Option<std::io::Error>>>,
    pub copy_result: Arc<Mutex<Option<std::io::Error>>>,
    pub remove_source_result: Arc<Mutex<Option<std::io::Error>>>,
    pub remove_staging_result: Arc<Mutex<Option<std::io::Error>>>,
    pub renamed_calls: Arc<Mutex<usize>>,
    pub copied_calls: Arc<Mutex<usize>>,
    pub removed_source_calls: Arc<Mutex<usize>>,
    pub removed_staging_calls: Arc<Mutex<usize>>,
}

impl FakeFileOps {
    fn new(expected_source_path: std::path::PathBuf) -> Self {
        Self {
            expected_source_path,
            rename_result: Default::default(),
            copy_result: Default::default(),
            remove_source_result: Default::default(),
            remove_staging_result: Default::default(),
            renamed_calls: Default::default(),
            copied_calls: Default::default(),
            removed_source_calls: Default::default(),
            removed_staging_calls: Default::default(),
        }
    }
}

#[async_trait::async_trait]
impl FileOps for FakeFileOps {
    async fn rename(&self, _from: &Path, _to: &Path) -> std::io::Result<()> {
        *self.renamed_calls.lock().await += 1;
        if let Some(err) = self.rename_result.lock().await.take() {
            return Err(err);
        }
        Ok(())
    }

    async fn copy(&self, from: &Path, to: &Path) -> std::io::Result<u64> {
        *self.copied_calls.lock().await += 1;
        if let Some(err) = self.copy_result.lock().await.take() {
            return Err(err);
        }
        // Simulate actual copy so metadata works
        tokio::fs::copy(from, to).await
    }

    async fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        let is_source = path == self.expected_source_path;
        if is_source {
            *self.removed_source_calls.lock().await += 1;
            if let Some(err) = self.remove_source_result.lock().await.take() {
                return Err(err);
            }
        } else {
            *self.removed_staging_calls.lock().await += 1;
            if let Some(err) = self.remove_staging_result.lock().await.take() {
                return Err(err);
            }
        }
        tokio::fs::remove_file(path).await
    }
}

#[tokio::test]
async fn test_source_remove_failure_after_fallback_copy() {
    let temp_dir = tempdir().unwrap();
    let base_dir = temp_dir.path();
    let source_file = base_dir.join("source.txt");
    std::fs::write(&source_file, "data").unwrap();

    let fake_ops = FakeFileOps::new(source_file.clone());
    *fake_ops.rename_result.lock().await = Some(std::io::Error::other("rename failed"));
    *fake_ops.remove_source_result.lock().await =
        Some(std::io::Error::other("remove source failed"));

    let result = stage_owned_temp_file_with_ops(
        base_dir,
        &ProjectId::new(),
        ArtifactKind::FinalVideo,
        &source_file,
        None,
        &fake_ops,
    )
    .await;

    let err_msg = match result {
        Err(PortError::Io { message }) => message,
        _ => panic!("Expected IO error"),
    };

    assert!(err_msg.contains("remove source failed"));
    assert!(err_msg.contains("Staging copy rolled back"));

    assert_eq!(*fake_ops.renamed_calls.lock().await, 1);
    assert_eq!(*fake_ops.copied_calls.lock().await, 1);
    assert_eq!(*fake_ops.removed_source_calls.lock().await, 1);
    assert_eq!(*fake_ops.removed_staging_calls.lock().await, 1);
}

#[tokio::test]
async fn test_source_remove_and_staging_remove_both_fail() {
    let temp_dir = tempdir().unwrap();
    let base_dir = temp_dir.path();
    let source_file = base_dir.join("source.txt");
    std::fs::write(&source_file, "data").unwrap();

    let fake_ops = FakeFileOps::new(source_file.clone());
    *fake_ops.rename_result.lock().await = Some(std::io::Error::other("rename failed"));
    *fake_ops.remove_source_result.lock().await =
        Some(std::io::Error::other("remove source failed"));
    *fake_ops.remove_staging_result.lock().await =
        Some(std::io::Error::other("remove staging failed"));

    let result = stage_owned_temp_file_with_ops(
        base_dir,
        &ProjectId::new(),
        ArtifactKind::FinalVideo,
        &source_file,
        None,
        &fake_ops,
    )
    .await;

    let err_msg = match result {
        Err(PortError::Io { message }) => message,
        _ => panic!("Expected IO error"),
    };

    assert!(err_msg.contains("remove source failed"));
    assert!(err_msg.contains("remove staging failed"));
}

#[tokio::test]
async fn test_copy_fails() {
    let temp_dir = tempdir().unwrap();
    let base_dir = temp_dir.path();
    let source_file = base_dir.join("source.txt");
    std::fs::write(&source_file, "data").unwrap();

    let fake_ops = FakeFileOps::new(source_file.clone());
    *fake_ops.rename_result.lock().await = Some(std::io::Error::other("rename failed"));
    *fake_ops.copy_result.lock().await = Some(std::io::Error::other("copy failed"));

    let result = stage_owned_temp_file_with_ops(
        base_dir,
        &ProjectId::new(),
        ArtifactKind::FinalVideo,
        &source_file,
        None,
        &fake_ops,
    )
    .await;

    let err_msg = match result {
        Err(PortError::Io { message }) => message,
        _ => panic!("Expected IO error"),
    };

    assert!(err_msg.contains("rename failed"));
    assert!(err_msg.contains("copy failed"));
    // Staging successfully deleted, so its error is NOT present
    assert!(!err_msg.contains("Rollback of staging copy also failed"));

    assert_eq!(*fake_ops.renamed_calls.lock().await, 1);
    assert_eq!(*fake_ops.copied_calls.lock().await, 1);
    assert_eq!(*fake_ops.removed_source_calls.lock().await, 1);
    assert_eq!(*fake_ops.removed_staging_calls.lock().await, 1);
}

#[tokio::test]
async fn test_copy_fails_and_staging_remove_fails() {
    let temp_dir = tempdir().unwrap();
    let base_dir = temp_dir.path();
    let source_file = base_dir.join("source.txt");
    std::fs::write(&source_file, "data").unwrap();

    let fake_ops = FakeFileOps::new(source_file.clone());
    *fake_ops.rename_result.lock().await = Some(std::io::Error::other("rename failed"));
    *fake_ops.copy_result.lock().await = Some(std::io::Error::other("copy failed"));
    *fake_ops.remove_staging_result.lock().await =
        Some(std::io::Error::other("staging remove failed"));

    let result = stage_owned_temp_file_with_ops(
        base_dir,
        &ProjectId::new(),
        ArtifactKind::FinalVideo,
        &source_file,
        None,
        &fake_ops,
    )
    .await;

    let err_msg = match result {
        Err(PortError::Io { message }) => message,
        _ => panic!("Expected IO error"),
    };

    assert!(err_msg.contains("rename failed"));
    assert!(err_msg.contains("copy failed"));
    assert!(err_msg.contains("staging remove failed"));
    assert!(err_msg.contains("Rollback of staging copy also failed"));

    assert_eq!(*fake_ops.renamed_calls.lock().await, 1);
    assert_eq!(*fake_ops.copied_calls.lock().await, 1);
    assert_eq!(*fake_ops.removed_source_calls.lock().await, 1);
    assert_eq!(*fake_ops.removed_staging_calls.lock().await, 1);
}

#[tokio::test]
async fn test_full_fallback_success() {
    let temp_dir = tempdir().unwrap();
    let base_dir = temp_dir.path();
    let source_file = base_dir.join("source.txt");
    std::fs::write(&source_file, "data").unwrap();

    let fake_ops = FakeFileOps::new(source_file.clone());
    *fake_ops.rename_result.lock().await = Some(std::io::Error::other("rename failed"));

    let result = stage_owned_temp_file_with_ops(
        base_dir,
        &ProjectId::new(),
        ArtifactKind::FinalVideo,
        &source_file,
        None,
        &fake_ops,
    )
    .await;

    assert!(result.is_ok());

    assert_eq!(*fake_ops.renamed_calls.lock().await, 1);
    assert_eq!(*fake_ops.copied_calls.lock().await, 1);
    assert_eq!(*fake_ops.removed_source_calls.lock().await, 1);
    assert_eq!(*fake_ops.removed_staging_calls.lock().await, 0); // Staging should be kept
}
