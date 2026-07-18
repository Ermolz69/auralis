#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::path::PathBuf;
use std::sync::Arc;

use domain::media::MediaMetadata;
use domain::project::{Project, ProjectId, ProjectStatus};
use ports::media::MediaProbePort;
use ports::repository::ProjectRepository;

use super::*;
use crate::error::ApplicationError;
use crate::test_utils::MockStorageUnitOfWork;
use crate::usecases::project::lifecycle::ProjectLifecycleLocks;
use adapters_storage::memory::{InMemoryDatabase, InMemoryProjectRepository};

#[derive(Clone, Default)]
pub(super) struct MockProbe;

#[async_trait::async_trait]
impl MediaProbePort for MockProbe {
    async fn probe_local_file(
        &self,
        _path: &std::path::Path,
    ) -> Result<MediaMetadata, ports::error::PortError> {
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

#[derive(Clone, Default)]
pub(super) struct MockArtifactStore {
    pub(super) artifacts: Arc<std::sync::Mutex<Vec<String>>>,
    pub(super) should_fail_delete: Arc<std::sync::atomic::AtomicBool>,
}

impl MockArtifactStore {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn set_should_fail_delete(&self, val: bool) {
        self.should_fail_delete
            .store(val, std::sync::atomic::Ordering::SeqCst);
    }
}

#[async_trait::async_trait]
impl ports::storage::ArtifactStore for MockArtifactStore {
    async fn stage_owned_temp_file(
        &self,
        _project_id: &ProjectId,
        _kind: domain::media::ArtifactKind,
        _temp_path: &std::path::Path,
        _filename_hint: Option<&str>,
    ) -> Result<ports::storage::StagedArtifact, ports::error::PortError> {
        unimplemented!()
    }

    async fn import_external_file(
        &self,
        project_id: &ProjectId,
        kind: domain::media::ArtifactKind,
        _source_path: &std::path::Path,
        filename_hint: Option<&str>,
    ) -> Result<ports::storage::StagedArtifact, ports::error::PortError> {
        let artifact_id = domain::media::ArtifactId::new();
        let filename = filename_hint.unwrap_or("video.mp4");
        let staging_key = format!(".staging/uuid/{filename}");
        let final_key = format!("{}/source-video/{}.mp4", project_id, artifact_id);

        self.artifacts.lock().unwrap().push(staging_key.clone());

        let artifact = domain::media::Artifact {
            id: artifact_id,
            kind,
            location: domain::media::ArtifactLocation::StorageKey(final_key.clone()),
            size_bytes: Some(1024),
            state: domain::media::ArtifactState::PendingFinalize,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            ready_at: None,
        };

        Ok(ports::storage::StagedArtifact {
            artifact,
            staging_key,
            final_key,
            size_bytes: 1024,
        })
    }

    async fn cleanup_stale_staging(
        &self,
        _max_age: std::time::Duration,
    ) -> Result<(), ports::error::PortError> {
        unimplemented!()
    }

    async fn finalize_staged_artifact(
        &self,
        _staging_key: &str,
        _final_key: &str,
    ) -> Result<(), ports::error::PortError> {
        unimplemented!()
    }

    async fn resolve_artifact(
        &self,
        _artifact: &domain::media::Artifact,
    ) -> Result<std::path::PathBuf, ports::error::PortError> {
        unimplemented!()
    }

    async fn delete_storage_key(&self, key: &str) -> Result<(), ports::error::PortError> {
        if self
            .should_fail_delete
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            return Err(ports::error::PortError::Io {
                message: "Failed to delete staged file".to_string(),
            });
        }
        let mut list = self.artifacts.lock().unwrap();
        list.retain(|k| k != key);
        Ok(())
    }

    async fn delete_artifact(
        &self,
        _artifact: &domain::media::Artifact,
    ) -> Result<(), ports::error::PortError> {
        unimplemented!()
    }

    async fn delete_project_dir(
        &self,
        _project_id: &ProjectId,
    ) -> Result<(), ports::error::PortError> {
        unimplemented!()
    }
}

pub(super) fn create_temp_file(dir: &tempfile::TempDir) -> PathBuf {
    let file_path = dir.path().join("video.mp4");
    std::fs::File::create(&file_path).unwrap();
    file_path
}

#[tokio::test]
async fn test_missing_project_before_stage() {
    let repo =
        InMemoryProjectRepository::new(Arc::new(std::sync::Mutex::new(InMemoryDatabase::new())));
    let probe = MockProbe;
    let uow = Arc::new(MockStorageUnitOfWork::new());
    let store = MockArtifactStore::new();
    let locks = Arc::new(ProjectLifecycleLocks::new());

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = create_temp_file(&temp_dir);

    let use_case = ImportLocalMediaUseCase::new(repo, probe, uow, store.clone(), locks);

    let req = ImportLocalMediaRequest {
        project_id: ProjectId::new(),
        path: file_path,
    };

    let err = use_case.execute(req).await.unwrap_err();
    assert!(matches!(err, ApplicationError::ProjectNotFound(_)));

    let artifacts = store.artifacts.lock().unwrap();
    assert!(artifacts.is_empty());
}

#[tokio::test]
async fn test_successful_import_returns_and_persists_ready() {
    let repo =
        InMemoryProjectRepository::new(Arc::new(std::sync::Mutex::new(InMemoryDatabase::new())));
    let project = Project::new("Import Test".to_string());
    let project_id = project.id().clone();
    repo.create(project).await.unwrap();

    let probe = MockProbe;
    let uow = Arc::new(MockStorageUnitOfWork::new());
    let store = MockArtifactStore::new();
    let locks = Arc::new(ProjectLifecycleLocks::new());

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = create_temp_file(&temp_dir);

    let use_case =
        ImportLocalMediaUseCase::new(repo.clone(), probe, uow.clone(), store.clone(), locks);

    let req = ImportLocalMediaRequest {
        project_id: project_id.clone(),
        path: file_path,
    };

    let res = use_case.execute(req).await.unwrap();
    assert_eq!(res.project.status(), &ProjectStatus::ReadyForProcessing);

    let saved_projects = uow.projects_saved.lock().await;
    assert_eq!(saved_projects.len(), 1);
    assert_eq!(
        saved_projects[0].status(),
        &ProjectStatus::ReadyForProcessing
    );
}

#[tokio::test]
async fn test_invalid_domain_transition_non_draft_rejected() {
    let repo =
        InMemoryProjectRepository::new(Arc::new(std::sync::Mutex::new(InMemoryDatabase::new())));
    let mut project = Project::new("Ready Project".to_string());
    project
        .import_source(
            domain::media::MediaSource::ManagedLocalFile {
                artifact_id: domain::media::ArtifactId::new(),
                original_filename: "test.mp4".into(),
            },
            None,
        )
        .unwrap();
    project.mark_ready_for_processing().unwrap();
    let project_id = project.id().clone();
    repo.create(project).await.unwrap();

    let probe = MockProbe;
    let uow = Arc::new(MockStorageUnitOfWork::new());
    let store = MockArtifactStore::new();
    let locks = Arc::new(ProjectLifecycleLocks::new());

    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = create_temp_file(&temp_dir);

    let use_case = ImportLocalMediaUseCase::new(repo, probe, uow, store, locks);

    let req = ImportLocalMediaRequest {
        project_id,
        path: file_path,
    };

    let err = use_case.execute(req).await.unwrap_err();
    assert!(matches!(err, ApplicationError::InvalidOperation { .. }));
}
