use async_trait::async_trait;
use domain::project::ProjectId;

use super::YoutubeImportSession;
use crate::{error::PortError, transaction::CommitYoutubeImport};

#[async_trait]
pub trait YoutubeImportJournal: Send + Sync {
    async fn find(&self, request_key: &str) -> Result<Option<YoutubeImportSession>, PortError>;
    async fn list(&self) -> Result<Vec<YoutubeImportSession>, PortError>;
    async fn protected_staging_keys(&self) -> Result<Vec<String>, PortError> {
        Ok(self
            .list()
            .await?
            .into_iter()
            .filter_map(|session| session.write.map(|write| write.staging_key))
            .collect())
    }
    async fn insert(&self, session: &YoutubeImportSession) -> Result<(), PortError>;
    async fn checkpoint(&self, session: &mut YoutubeImportSession) -> Result<(), PortError>;
    async fn commit(
        &self,
        session: &YoutubeImportSession,
        command: CommitYoutubeImport,
    ) -> Result<(), PortError>;
    async fn discard(&self, project_id: &ProjectId, revision: u64) -> Result<(), PortError>;
}
