use super::fixture::checkpoint;
use adapters_storage::sqlite::youtube_import_journal::SqliteYoutubeImportJournal;
use async_trait::async_trait;
use domain::project::ProjectId;
use ports::{
    error::PortError,
    transaction::CommitYoutubeImport,
    youtube_import::{YoutubeImportJournal, YoutubeImportSession},
};
use std::{path::PathBuf, sync::Arc};

pub(super) struct CrashJournal {
    pub inner: Arc<SqliteYoutubeImportJournal>,
    pub root: PathBuf,
}

#[async_trait]
impl YoutubeImportJournal for CrashJournal {
    async fn find(&self, key: &str) -> Result<Option<YoutubeImportSession>, PortError> {
        self.inner.find(key).await
    }
    async fn list(&self) -> Result<Vec<YoutubeImportSession>, PortError> {
        self.inner.list().await
    }
    async fn insert(&self, session: &YoutubeImportSession) -> Result<(), PortError> {
        self.inner.insert(session).await
    }
    async fn checkpoint(&self, session: &mut YoutubeImportSession) -> Result<(), PortError> {
        if session.write.is_some() {
            checkpoint(&self.root, "stage-before-checkpoint");
        }
        self.inner.checkpoint(session).await?;
        if session.write.is_some() {
            checkpoint(&self.root, "stage-after-checkpoint");
        }
        Ok(())
    }
    async fn commit(
        &self,
        session: &YoutubeImportSession,
        command: CommitYoutubeImport,
    ) -> Result<(), PortError> {
        checkpoint(&self.root, "before-commit");
        self.inner.commit(session, command).await?;
        checkpoint(&self.root, "after-commit");
        Ok(())
    }
    async fn discard(&self, id: &ProjectId, revision: u64) -> Result<(), PortError> {
        self.inner.discard(id, revision).await
    }
}
