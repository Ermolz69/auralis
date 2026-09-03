#![allow(clippy::unwrap_used)]

use super::{
    create_from_youtube::CreateProjectFromYoutubeUseCase, lifecycle::ProjectLifecycleLocks,
    youtube_storage_support::FileVideoSource,
};
use adapters_storage::{
    local::{LocalArtifactStore, LocalTempWorkspace},
    sqlite::{SqliteProjectRepository, SqliteStorageUnitOfWork, connect_sqlite},
};
use async_trait::async_trait;
use domain::media::{Artifact, MediaMetadata, MediaSource};
use domain::project::Project;
use ports::{
    error::PortError,
    repository::ProjectRepository,
    source::{DownloadMediaRequest, VideoSourcePort},
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU8, Ordering},
};
use tokio::sync::Notify;

#[derive(Clone, Default)]
pub(super) struct ControlledSource {
    pub failure: Arc<AtomicU8>,
    pub reached: Arc<Notify>,
    pub released: Arc<Notify>,
    pub paths: Arc<Mutex<Vec<std::path::PathBuf>>>,
}

impl ControlledSource {
    fn check(&self, phase: u8) -> Result<(), PortError> {
        if self.failure.load(Ordering::SeqCst) == phase {
            return Err(PortError::Network {
                message: "Injected source failure".into(),
            });
        }
        Ok(())
    }
    pub async fn wait(&self) {
        tokio::time::timeout(std::time::Duration::from_secs(10), self.reached.notified())
            .await
            .unwrap();
    }
    fn inner(&self) -> FileVideoSource {
        FileVideoSource {
            extension: "mp4",
            downloaded_paths: self.paths.clone(),
        }
    }
}

#[async_trait]
impl VideoSourcePort for ControlledSource {
    async fn validate_source(&self, source: &MediaSource) -> Result<(), PortError> {
        self.check(1)?;
        self.inner().validate_source(source).await
    }
    async fn fetch_metadata(&self, source: &MediaSource) -> Result<MediaMetadata, PortError> {
        self.check(2)?;
        self.inner().fetch_metadata(source).await
    }
    async fn download_media(&self, request: DownloadMediaRequest) -> Result<Artifact, PortError> {
        let artifact = self.inner().download_media(request).await?;
        self.check(3)?;
        if self.failure.load(Ordering::SeqCst) == 4 {
            let path = self.paths.lock().unwrap().last().unwrap().clone();
            tokio::fs::remove_file(path).await.unwrap();
        }
        if self.failure.load(Ordering::SeqCst) == 5 {
            self.reached.notify_one();
            tokio::time::timeout(std::time::Duration::from_secs(10), self.released.notified())
                .await
                .unwrap();
        }
        Ok(artifact)
    }
}

pub(super) struct Fixture {
    pub root: tempfile::TempDir,
    pub pool: sqlx::SqlitePool,
    pub repo: Arc<SqliteProjectRepository>,
    pub store: Arc<LocalArtifactStore>,
    pub workspace: Arc<LocalTempWorkspace>,
    pub source: ControlledSource,
    pub locks: Arc<ProjectLifecycleLocks>,
}

impl Fixture {
    pub async fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let pool = connect_sqlite(root.path().join("test.db")).await.unwrap();
        Self {
            repo: Arc::new(SqliteProjectRepository::new(pool.clone())),
            store: Arc::new(LocalArtifactStore::new(root.path().join("artifacts"))),
            workspace: Arc::new(LocalTempWorkspace::new(root.path().to_path_buf())),
            root,
            pool,
            source: ControlledSource::default(),
            locks: Arc::new(ProjectLifecycleLocks::new()),
        }
    }
    pub fn usecase(
        &self,
    ) -> CreateProjectFromYoutubeUseCase<
        Arc<SqliteProjectRepository>,
        ControlledSource,
        Arc<LocalArtifactStore>,
        Arc<SqliteStorageUnitOfWork>,
    > {
        CreateProjectFromYoutubeUseCase::new(
            self.repo.clone(),
            self.source.clone(),
            self.store.clone(),
            Arc::new(SqliteStorageUnitOfWork::new(self.pool.clone())),
            self.workspace.clone(),
            self.locks.clone(),
        )
    }
    pub async fn assert_unchanged(&self, original: Option<&Project>) {
        assert_eq!(
            self.repo.list().await.unwrap(),
            original.cloned().into_iter().collect::<Vec<_>>()
        );
        for query in [
            "SELECT COUNT(*) FROM artifacts",
            "SELECT COUNT(*) FROM outbox_messages",
        ] {
            let count: i64 = sqlx::query_scalar(query)
                .fetch_one(&self.pool)
                .await
                .unwrap();
            assert_eq!(count, 0, "{query} must return no rolled-back writes");
        }
        for path in self.source.paths.lock().unwrap().iter() {
            assert!(
                !path.parent().unwrap().exists(),
                "allocation must be removed"
            );
        }
        assert_no_files(&self.root.path().join("artifacts"));
    }
}

fn assert_no_files(path: &std::path::Path) {
    if !path.exists() {
        return;
    }
    for entry in std::fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        assert!(
            entry.file_type().unwrap().is_dir(),
            "unexpected staged file: {:?}",
            entry.path()
        );
        assert_no_files(&entry.path());
    }
}
