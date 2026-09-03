#![allow(clippy::unwrap_used)]

use super::super::{
    create_from_youtube::CreateProjectFromYoutubeUseCase, lifecycle::ProjectLifecycleLocks,
};
use adapters_storage::{
    local::{LocalArtifactStore, LocalTempWorkspace},
    sqlite::{
        SqliteProjectRepository, connect_sqlite, youtube_import_journal::SqliteYoutubeImportJournal,
    },
};
use adapters_ytdlp::mock::MockVideoSourceAdapter;
use async_trait::async_trait;
use domain::media::{Artifact, MediaMetadata, MediaSource};
use ports::{
    error::PortError,
    source::{DownloadMediaRequest, VideoSourcePort},
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub(super) const URL: &str = "https://youtube.com/watch?v=resume";

pub(super) fn checkpoint(root: &Path, phase: &str) {
    if std::env::var("AURALIS_CRASH_PHASE").ok().as_deref() == Some(phase) {
        std::fs::write(root.join("checkpoint"), phase).unwrap();
        loop {
            std::thread::park();
        }
    }
}

#[derive(Clone)]
pub(super) struct Source(pub PathBuf);

#[async_trait]
impl VideoSourcePort for Source {
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
        let output = request.target_dir.join("original.mp4");
        let partial = request.target_dir.join("original.mp4.part");
        if !output.exists() {
            if !partial.exists() {
                std::fs::write(&partial, b"partial ").unwrap();
                checkpoint(&self.0, "download");
            }
            assert_eq!(std::fs::read(&partial).unwrap(), b"partial ");
            std::fs::write(&output, b"partial video").unwrap();
            std::fs::remove_file(&partial).unwrap();
            let counter = self.0.join("network-completions");
            assert!(
                !counter.exists(),
                "completed media must never be downloaded twice"
            );
            std::fs::write(counter, b"1").unwrap();
        }
        request.filename_hint = Some("original.mp4".into());
        MockVideoSourceAdapter::new().download_media(request).await
    }
}

pub(super) struct Fixture {
    pub root: PathBuf,
    pub pool: sqlx::SqlitePool,
    pub repo: Arc<SqliteProjectRepository>,
    pub store: Arc<LocalArtifactStore>,
    pub workspace: Arc<LocalTempWorkspace>,
    pub journal: Arc<SqliteYoutubeImportJournal>,
}

impl Fixture {
    pub async fn open(root: &Path) -> Self {
        let pool = connect_sqlite(root.join("resume.sqlite")).await.unwrap();
        Self {
            root: root.into(),
            repo: Arc::new(SqliteProjectRepository::new(pool.clone())),
            store: Arc::new(LocalArtifactStore::new(root.join("artifacts"))),
            workspace: Arc::new(LocalTempWorkspace::new(root)),
            journal: Arc::new(SqliteYoutubeImportJournal::new(pool.clone())),
            pool,
        }
    }
    pub fn usecase(
        &self,
    ) -> CreateProjectFromYoutubeUseCase<
        Arc<SqliteProjectRepository>,
        Source,
        Arc<LocalArtifactStore>,
    > {
        CreateProjectFromYoutubeUseCase::new(
            self.repo.clone(),
            Source(self.root.clone()),
            self.store.clone(),
            Arc::new(super::journal::CrashJournal {
                inner: self.journal.clone(),
                root: self.root.clone(),
            }),
            self.workspace.clone(),
            Arc::new(ProjectLifecycleLocks::new()),
        )
    }
}
