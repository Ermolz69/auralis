use domain::{
    outbox::{OutboxMessage, OutboxPayload},
    project::ProjectId,
};
use ports::error::PortError;
use sqlx::{Sqlite, Transaction};

use super::mapping::{conflict, decode};
use crate::sqlite::{helpers::map_sqlite_error, unit_of_work::outbox_writes::save_outbox_message};

pub(crate) async fn discard_in_transaction(
    tx: &mut Transaction<'_, Sqlite>,
    id: &ProjectId,
    revision: Option<u64>,
) -> Result<(), PortError> {
    let row = sqlx::query_as("SELECT project_id, request_key, revision, payload_json FROM youtube_imports WHERE project_id = ?")
        .bind(id.to_string()).fetch_optional(&mut **tx).await.map_err(|e| map_sqlite_error("discard_youtube_import", e))?;
    let Some(row) = row else {
        return if revision.is_some() {
            Err(conflict())
        } else {
            Ok(())
        };
    };
    let session = decode(row)?;
    if revision.is_some_and(|revision| revision != session.revision) {
        return Err(conflict());
    }
    if let Some(write) = session.write {
        let storage_key = write
            .staging_key
            .parse()
            .map_err(|_| super::mapping::invalid())?;
        save_outbox_message(
            tx,
            &OutboxMessage::new(OutboxPayload::DeleteStorageKey { storage_key }),
        )
        .await?;
    }
    save_outbox_message(
        tx,
        &OutboxMessage::new(OutboxPayload::DeleteWorkspaceAllocation {
            workspace_key: session.workspace_key,
        }),
    )
    .await?;
    sqlx::query("DELETE FROM youtube_imports WHERE project_id = ?")
        .bind(id.to_string())
        .execute(&mut **tx)
        .await
        .map_err(|e| map_sqlite_error("discard_youtube_import", e))?;
    Ok(())
}
