use std::path::PathBuf;

use domain::media::ArtifactKind;
use domain::project::ProjectId;
use ports::repository::ProjectRepository;
use ports::source::{DownloadMediaRequest, VideoSourcePort};
use ports::storage::ArtifactStore;
use ports::transaction::{CommitMediaDownload, StorageUnitOfWork};

use crate::error::ApplicationError;

#[derive(Debug)]
pub struct DownloadYoutubeVideoRequest {
    pub project_id: ProjectId,
    pub temp_dir: PathBuf,
    pub filename_hint: Option<String>,
}

pub struct DownloadYoutubeVideoUseCase<P, V, S, T>
where
    P: ProjectRepository,
    V: VideoSourcePort,
    S: ArtifactStore,
    T: StorageUnitOfWork,
{
    project_repo: P,
    video_source: V,
    artifact_store: S,
    storage_uow: T,
}

impl<P, V, S, T> DownloadYoutubeVideoUseCase<P, V, S, T>
where
    P: ProjectRepository,
    V: VideoSourcePort,
    S: ArtifactStore,
    T: StorageUnitOfWork,
{
    pub fn new(
        project_repo: P,
        video_source: V,
        artifact_store: S,
        storage_uow: T,
    ) -> Self {
        Self {
            project_repo,
            video_source,
            artifact_store,
            storage_uow,
        }
    }

    pub async fn execute(
        &self,
        request: DownloadYoutubeVideoRequest,
    ) -> Result<(), ApplicationError> {
        let project = self
            .project_repo
            .get(&request.project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(request.project_id.clone()))?;

        let source = project
            .source()
            .ok_or_else(|| ApplicationError::InvalidOperation {
                message: "Project has no media source".into(),
            })?;

        if !matches!(
            source,
            domain::media::MediaSource::YoutubeUrl { .. }
                | domain::media::MediaSource::RemoteUrl { .. }
        ) {
            return Err(ApplicationError::InvalidOperation {
                message: "Source is not a remote URL or YouTube URL".into(),
            });
        }

        std::fs::create_dir_all(&request.temp_dir).map_err(|e| {
            ApplicationError::InvalidOperation {
                message: format!("Failed to create temp directory: {}", e),
            }
        })?;

        let download_req = DownloadMediaRequest {
            source: source.clone(),
            target_dir: request.temp_dir.clone(),
            filename_hint: request.filename_hint.clone(),
        };

        // 1. Download to temporary path
        let temp_artifact = self.video_source.download_media(download_req).await?;

        let temp_path = match temp_artifact.location {
            domain::media::ArtifactLocation::LocalPath(p) => std::path::PathBuf::from(p),
            domain::media::ArtifactLocation::StorageKey(_) => {
                return Err(ApplicationError::InvalidOperation {
                    message: "Expected LocalPath from download_media, got StorageKey".into(),
                });
            }
        };

        // 2. Stage artifact in the ArtifactStore
        let staged = match self
            .artifact_store
            .stage_external_file(
                &request.project_id,
                ArtifactKind::DownloadedVideo,
                &temp_path,
                request.filename_hint.as_deref(),
            )
            .await
        {
            Ok(s) => s,
            Err(e) => {
                let _ = std::fs::remove_file(&temp_path);
                return Err(ApplicationError::Port(e));
            }
        };

        // 3. Atomically persist to DB and write outbox message
        let commit_cmd = CommitMediaDownload {
            project_id: request.project_id.clone(),
            artifact: staged.artifact,
            staging_key: staged.staging_key.clone(),
            final_key: staged.final_key.clone(),
            temp_path_to_delete: Some(temp_path),
        };

        if let Err(e) = self.storage_uow.commit_media_download(commit_cmd).await {
            // DB failed, we can optionally clean up the staging file
            let _ = self
                .artifact_store
                .delete_storage_key(&staged.staging_key)
                .await;
            return Err(ApplicationError::Port(e));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapters_storage::memory::InMemoryProjectRepository;
    use adapters_ytdlp::mock::MockVideoSourceAdapter;
    use async_trait::async_trait;
    use domain::media::{Artifact, ArtifactId, ArtifactKind, ArtifactLocation, MediaSource};
    use domain::project::Project;
    use ports::error::PortError;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use crate::test_utils::MockStorageUnitOfWork;

    #[derive(Clone)]
    struct MockArtifactStore {
        fail_on_stage: Arc<AtomicBool>,
        deleted_keys: Arc<std::sync::Mutex<Vec<String>>>,
    }

    #[async_trait]
    impl ArtifactStore for MockArtifactStore {
        async fn write_small_artifact(
            &self,
            _project_id: &ProjectId,
            _kind: ArtifactKind,
            _filename: &str,
            _data: &[u8],
        ) -> Result<Artifact, PortError> {
            unimplemented!()
        }

        async fn stage_external_file(
            &self,
            _project_id: &ProjectId,
            _kind: ArtifactKind,
            source_path: &std::path::Path,
            filename_hint: Option<&str>,
        ) -> Result<ports::storage::StagedArtifact, PortError> {
            if self.fail_on_stage.load(Ordering::SeqCst) {
                return Err(PortError::Io {
                    message: "Simulated stage error".into(),
                });
            }

            let ext = filename_hint
                .and_then(|h| std::path::Path::new(h).extension())
                .or_else(|| source_path.extension())
                .and_then(|e| e.to_str())
                .unwrap_or("bin");

            let artifact = Artifact {
                id: ArtifactId::new(),
                kind: ArtifactKind::DownloadedVideo,
                location: ArtifactLocation::StorageKey(format!("final.{}", ext)),
                size_bytes: Some(1024),
                state: domain::media::ArtifactState::PendingFinalize,
                created_at: domain::chrono::Utc::now(),
                updated_at: domain::chrono::Utc::now(),
                ready_at: None,
            };

            Ok(ports::storage::StagedArtifact {
                artifact,
                staging_key: format!("staging.{}", ext),
                final_key: format!("final.{}", ext),
                size_bytes: 1024,
            })
        }

        async fn finalize_staged_artifact(
            &self,
            _staging_key: &str,
            _final_key: &str,
        ) -> Result<(), PortError> {
            Ok(())
        }

        async fn resolve_artifact(&self, _artifact: &Artifact) -> Result<PathBuf, PortError> {
            Ok(PathBuf::from("/tmp/artifact"))
        }

        async fn delete_storage_key(&self, storage_key: &str) -> Result<(), PortError> {
            self.deleted_keys
                .lock()
                .unwrap()
                .push(storage_key.to_string());
            Ok(())
        }

        async fn delete_artifact(&self, _artifact: &Artifact) -> Result<(), PortError> {
            Ok(())
        }

        async fn delete_project_dir(&self, _project_id: &ProjectId) -> Result<(), PortError> {
            Ok(())
        }
    }

    #[derive(Clone)]
    struct FailingVideoSourceAdapter;

    #[async_trait]
    impl VideoSourcePort for FailingVideoSourceAdapter {
        async fn validate_source(&self, _source: &MediaSource) -> Result<(), PortError> {
            Ok(())
        }
        async fn fetch_metadata(
            &self,
            _source: &MediaSource,
        ) -> Result<domain::media::MediaMetadata, PortError> {
            unimplemented!()
        }
        async fn download_media(
            &self,
            _request: DownloadMediaRequest,
        ) -> Result<Artifact, PortError> {
            Ok(Artifact {
                id: ArtifactId::new(),
                kind: ArtifactKind::DownloadedVideo,
                location: ArtifactLocation::StorageKey("this_should_be_local_path".to_string()),
                size_bytes: None,
                state: domain::media::ArtifactState::Ready,
                created_at: domain::chrono::Utc::now(),
                updated_at: domain::chrono::Utc::now(),
                ready_at: Some(domain::chrono::Utc::now()),
            })
        }
    }

    #[tokio::test]
    async fn test_download_video_success() {
        let repo = InMemoryProjectRepository::new();
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

        let use_case = DownloadYoutubeVideoUseCase::new(
            repo,
            source_port,
            artifact_store,
            storage_uow,
        );

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
        let repo = InMemoryProjectRepository::new();
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
        let repo = InMemoryProjectRepository::new();
        let mut project = Project::new("Test Project".to_string());
        project
            .import_source(
                MediaSource::LocalFile {
                    path: "/tmp/test.mp4".into(),
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
        let repo = InMemoryProjectRepository::new();
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
        let repo = InMemoryProjectRepository::new();
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
        let repo = InMemoryProjectRepository::new();
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
}
