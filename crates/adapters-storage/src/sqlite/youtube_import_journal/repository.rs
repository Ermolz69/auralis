use async_trait::async_trait;
use domain::project::ProjectId;
use ports::{
    error::PortError,
    transaction::CommitYoutubeImport,
    youtube_import::{YoutubeImportJournal, YoutubeImportSession},
};
use sqlx::SqlitePool;

use super::mapping::{conflict, decode, invalid, validate};
use crate::sqlite::{helpers::map_sqlite_error, unit_of_work::youtube_import};

pub struct SqliteYoutubeImportJournal {
    pool: SqlitePool,
}

impl SqliteYoutubeImportJournal {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl YoutubeImportJournal for SqliteYoutubeImportJournal {
    async fn find(&self, request_key: &str) -> Result<Option<YoutubeImportSession>, PortError> {
        sqlx::query_as("SELECT project_id, request_key, revision, payload_json FROM youtube_imports WHERE request_key = ?")
            .bind(request_key).fetch_optional(&self.pool).await
            .map_err(|e| map_sqlite_error("find_youtube_import", e))?.map(decode).transpose()
    }

    async fn list(&self) -> Result<Vec<YoutubeImportSession>, PortError> {
        sqlx::query_as("SELECT project_id, request_key, revision, payload_json FROM youtube_imports ORDER BY project_id")
            .fetch_all(&self.pool).await.map_err(|e| map_sqlite_error("list_youtube_imports", e))?
            .into_iter().map(decode).collect()
    }

    async fn insert(&self, session: &YoutubeImportSession) -> Result<(), PortError> {
        validate(session)?;
        let payload = serde_json::to_string(session).map_err(|_| invalid())?;
        sqlx::query("INSERT INTO youtube_imports(project_id, request_key, revision, payload_json) VALUES (?, ?, ?, ?)")
            .bind(session.project.id.to_string()).bind(session.request_key()).bind(session.revision as i64).bind(payload)
            .execute(&self.pool).await.map_err(|e| match &e {
                sqlx::Error::Database(error) if error.is_unique_violation() => conflict(),
                _ => map_sqlite_error("insert_youtube_import", e),
            })?;
        Ok(())
    }

    async fn protected_staging_keys(&self) -> Result<Vec<String>, PortError> {
        let mut keys: Vec<_> = self
            .list()
            .await?
            .into_iter()
            .filter_map(|session| session.write.map(|write| write.staging_key))
            .collect();
        let payloads: Vec<String> = sqlx::query_scalar("SELECT payload_json FROM outbox_messages WHERE kind = 'finalize_staged_artifact' AND status != 'done'")
            .fetch_all(&self.pool).await.map_err(|e| map_sqlite_error("protect_pending_staging", e))?;
        for payload in payloads {
            match serde_json::from_str(&payload).map_err(|_| invalid())? {
                domain::outbox::OutboxPayload::FinalizeStagedArtifact { staging_key, .. } => {
                    keys.push(staging_key.to_string())
                }
                _ => return Err(invalid()),
            }
        }
        Ok(keys)
    }

    async fn checkpoint(&self, session: &mut YoutubeImportSession) -> Result<(), PortError> {
        validate(session)?;
        let mut next = session.clone();
        next.revision += 1;
        validate(&next)?;
        let payload = serde_json::to_string(&next).map_err(|_| invalid())?;
        let changed = sqlx::query("UPDATE youtube_imports SET payload_json = ?, revision = revision + 1 WHERE project_id = ? AND revision = ?")
            .bind(payload).bind(session.project.id.to_string()).bind(session.revision as i64)
            .execute(&self.pool).await.map_err(|e| map_sqlite_error("checkpoint_youtube_import", e))?.rows_affected();
        if changed != 1 {
            return Err(conflict());
        }
        *session = next;
        Ok(())
    }

    async fn commit(
        &self,
        session: &YoutubeImportSession,
        command: CommitYoutubeImport,
    ) -> Result<(), PortError> {
        validate(session)?;
        if command.project.to_snapshot() != session.project
            || command.original_updated_at != session.original_updated_at
            || session.write.as_ref() != Some(&command.write)
        {
            return Err(invalid());
        }
        let payload = serde_json::to_string(session).map_err(|_| invalid())?;
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|e| map_sqlite_error("begin_youtube_resume", e))?;
        let changed = sqlx::query("DELETE FROM youtube_imports WHERE project_id = ? AND revision = ? AND payload_json = ?")
            .bind(session.project.id.to_string()).bind(session.revision as i64).bind(payload)
            .execute(&mut *tx).await.map_err(|e| map_sqlite_error("complete_youtube_import", e))?.rows_affected();
        if changed != 1 {
            return Err(conflict());
        }
        youtube_import::write(&mut tx, command).await?;
        #[cfg(test)]
        super::tests::pause_before_commit();
        tx.commit()
            .await
            .map_err(|e| map_sqlite_error("commit_youtube_resume", e))
    }

    async fn discard(&self, project_id: &ProjectId, revision: u64) -> Result<(), PortError> {
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|e| map_sqlite_error("discard_youtube_import", e))?;
        super::discard::discard_in_transaction(&mut tx, project_id, Some(revision)).await?;
        tx.commit()
            .await
            .map_err(|e| map_sqlite_error("discard_youtube_import", e))
    }
}
