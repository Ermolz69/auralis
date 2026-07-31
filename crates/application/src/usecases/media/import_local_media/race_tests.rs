#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::sync::Arc;
use tokio::sync::Barrier;

use domain::media::MediaMetadata;
use domain::project::Project;
use ports::media::MediaProbePort;
use ports::repository::ProjectRepository;

use super::tests::{MockArtifactStore, MockProbe, create_temp_file};
use super::*;
use crate::error::ApplicationError;
use crate::test_utils::MockStorageUnitOfWork;
use crate::usecases::project::lifecycle::ProjectLifecycleLocks;
use adapters_storage::memory::{InMemoryDatabase, InMemoryProjectRepository};

#[derive(Clone)]
struct BarrierProbe {
    barrier: Arc<Barrier>,
}

#[async_trait::async_trait]
impl MediaProbePort for BarrierProbe {
    async fn probe_local_file(
        &self,
        _path: &std::path::Path,
    ) -> Result<MediaMetadata, ports::error::PortError> {
        self.barrier.wait().await;
        self.barrier.wait().await;
        Ok(MediaMetadata {
            duration_ms: 60000,
            width: Some(1920),
            height: Some(1080),
            fps: Some(30.0),
            video_codec: Some("h264".into()),
            audio_codec: Some("aac".into()),
            audio_channels: Some(2),
            sample_rate: Some(48000),
            container: Some("mp4".into()),
            bitrate: Some(128000),
            format_name: Some("mp4".into()),
            has_video: true,
            has_audio: true,
            streams: vec![],
            video: None,
            audio_tracks: vec![],
        })
    }
}

#[tokio::test]
async fn test_delete_during_probe() {
    let repo =
        InMemoryProjectRepository::new(Arc::new(std::sync::Mutex::new(InMemoryDatabase::new())));
    let project = Project::new("Delete during probe".to_string());
    let project_id = project.id().clone();
    repo.create(project).await.unwrap();

    let barrier = Arc::new(Barrier::new(2));
    let probe = BarrierProbe {
        barrier: barrier.clone(),
    };
    let uow = Arc::new(MockStorageUnitOfWork::new());
    let store = MockArtifactStore::new();
    let locks = Arc::new(ProjectLifecycleLocks::new());

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = create_temp_file(&temp_dir);

    let use_case = ImportLocalMediaUseCase::new(repo.clone(), probe, uow, store.clone(), locks);

    let req = ImportLocalMediaRequest {
        project_id: project_id.clone(),
        path: file_path,
    };

    let handle = tokio::spawn(async move { use_case.execute(req).await });

    barrier.wait().await;
    repo.delete(&project_id).await.unwrap();
    barrier.wait().await;

    let err = handle.await.unwrap().unwrap_err();
    assert!(matches!(err, ApplicationError::ProjectNotFound(_)));

    let artifacts = store.artifacts.lock().unwrap();
    assert!(artifacts.is_empty());
}

#[tokio::test]
async fn test_delete_after_stage_before_revalidation() {
    let repo =
        InMemoryProjectRepository::new(Arc::new(std::sync::Mutex::new(InMemoryDatabase::new())));
    let project = Project::new("Concurrent Delete".to_string());
    let project_id = project.id().clone();
    repo.create(project).await.unwrap();

    let probe = MockProbe;
    let uow = Arc::new(MockStorageUnitOfWork::new());
    let store = MockArtifactStore::new();
    let locks = Arc::new(ProjectLifecycleLocks::new());

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = create_temp_file(&temp_dir);

    let repo_clone = repo.clone();
    let project_id_clone = project_id.clone();
    repo_clone.delete(&project_id_clone).await.unwrap();

    let use_case = ImportLocalMediaUseCase::new(repo, probe, uow, store.clone(), locks);

    let req = ImportLocalMediaRequest {
        project_id,
        path: file_path,
    };

    let err = use_case.execute(req).await.unwrap_err();
    assert!(matches!(err, ApplicationError::ProjectNotFound(_)));

    let artifacts = store.artifacts.lock().unwrap();
    assert!(artifacts.is_empty());
}

#[tokio::test]
async fn test_delete_competing_with_db_phase_delete_wins() {
    let repo =
        InMemoryProjectRepository::new(Arc::new(std::sync::Mutex::new(InMemoryDatabase::new())));
    let project = Project::new("Delete Wins".to_string());
    let project_id = project.id().clone();
    repo.create(project).await.unwrap();

    let probe = MockProbe;
    let uow = Arc::new(MockStorageUnitOfWork::new());
    let store = MockArtifactStore::new();
    let locks = Arc::new(ProjectLifecycleLocks::new());

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = create_temp_file(&temp_dir);

    let lock_arc = locks.get_lock(&project_id).unwrap();
    {
        let _guard = lock_arc.lock().await;
        repo.delete(&project_id).await.unwrap();
    }

    let use_case = ImportLocalMediaUseCase::new(repo, probe, uow, store.clone(), locks);

    let req = ImportLocalMediaRequest {
        project_id,
        path: file_path,
    };

    let err = use_case.execute(req).await.unwrap_err();
    assert!(matches!(err, ApplicationError::ProjectNotFound(_)));

    let artifacts = store.artifacts.lock().unwrap();
    assert!(artifacts.is_empty());
}
