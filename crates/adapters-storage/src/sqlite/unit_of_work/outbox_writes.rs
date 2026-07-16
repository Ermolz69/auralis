use domain::outbox::{OutboxMessage, OutboxPayload};
use ports::error::PortError;
use sqlx::{Sqlite, Transaction};

pub(super) async fn save_outbox_message(
    tx: &mut Transaction<'_, Sqlite>,
    msg: &OutboxMessage,
) -> Result<(), PortError> {
    let kind = msg.payload.clone();
    let kind_str = match &kind {
        OutboxPayload::FinalizeStagedArtifact { .. } => "finalize_staged_artifact",
        OutboxPayload::DeleteStorageKey { .. } => "delete_storage_key",
        OutboxPayload::DeleteProjectArtifactDir { .. } => "delete_project_artifact_dir",
        OutboxPayload::DeleteWorkspaceFile { .. } => "delete_workspace_file",
        OutboxPayload::HandleTerminalJobState { .. } => "handle_terminal_job_state",
    };

    let payload_json = serde_json::to_string(&msg.payload).map_err(|e| PortError::Unexpected {
        message: format!("Failed to serialize outbox payload: {}", e),
    })?;

    let status_str = match msg.status {
        domain::outbox::OutboxMessageStatus::Pending => "pending",
        domain::outbox::OutboxMessageStatus::Processing => "processing",
        domain::outbox::OutboxMessageStatus::Done => "done",
        domain::outbox::OutboxMessageStatus::Failed => "failed",
        domain::outbox::OutboxMessageStatus::Dead => "dead",
    };

    sqlx::query(
        r#"
        INSERT OR IGNORE INTO outbox_messages (
            id, kind, payload_json, status, attempts, next_attempt_at,
            locked_at, locked_by, last_error, deduplication_key, created_at, updated_at,
            aggregate_type, aggregate_id
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(msg.id.to_string())
    .bind(kind_str)
    .bind(payload_json)
    .bind(status_str)
    .bind(msg.attempts)
    .bind(msg.next_attempt_at.to_rfc3339())
    .bind(msg.locked_at.map(|dt| dt.to_rfc3339()))
    .bind(msg.locked_by.clone())
    .bind(msg.last_error.clone())
    .bind(msg.deduplication_key.clone())
    .bind(msg.created_at.to_rfc3339())
    .bind(msg.updated_at.to_rfc3339())
    .bind(msg.aggregate_type.clone())
    .bind(msg.aggregate_id.clone())
    .execute(&mut **tx)
    .await
    .map_err(|e| {
        crate::sqlite::helpers::map_sqlite_error("Failed to add outbox message in tx", e)
    })?;
    Ok(())
}
