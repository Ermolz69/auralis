use domain::media::Artifact;
use ports::error::PortError;
use sqlx::{Sqlite, Transaction};

use crate::sqlite::artifact_index::mapper::artifact_to_row_values;

pub(super) async fn save_artifact(
    tx: &mut Transaction<'_, Sqlite>,
    project_id: &domain::project::ProjectId,
    artifact: &Artifact,
) -> Result<(), PortError> {
    let values = artifact_to_row_values(project_id, artifact)?;
    sqlx::query(
        r#"
        INSERT INTO artifacts (
            id, project_id, kind, location_kind, location_value, size_bytes, 
            state, created_at, updated_at, ready_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            project_id = excluded.project_id,
            kind = excluded.kind,
            location_kind = excluded.location_kind,
            location_value = excluded.location_value,
            size_bytes = excluded.size_bytes,
            state = excluded.state,
            updated_at = excluded.updated_at,
            ready_at = excluded.ready_at
        "#,
    )
    .bind(values.id)
    .bind(values.project_id)
    .bind(values.kind)
    .bind(values.location_kind)
    .bind(values.location_value)
    .bind(values.size_bytes)
    .bind(values.state)
    .bind(values.created_at)
    .bind(values.updated_at)
    .bind(values.ready_at)
    .execute(&mut **tx)
    .await
    .map_err(|e| crate::sqlite::helpers::map_sqlite_error("Failed to add artifact in tx", e))?;
    Ok(())
}

pub(super) async fn finalize_artifact(
    tx: &mut Transaction<'_, Sqlite>,
    message_id: &str,
    project_id: &domain::project::ProjectId,
    artifact_id: &domain::media::ArtifactId,
    ready_key: &str,
) -> Result<ports::transaction::CommitArtifactFinalizeResult, PortError> {
    // 1. Check outbox status
    let outbox_row = sqlx::query("SELECT status FROM outbox_messages WHERE id = ?")
        .bind(message_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to query outbox message status", e)
        })?;

    let outbox_status = match outbox_row {
        Some(row) => {
            use sqlx::Row;
            row.try_get::<String, _>("status")
                .map_err(|e| PortError::InvalidStoredData {
                    entity_type: "outbox".to_string(),
                    entity_id: message_id.to_string(),
                    field: "status".to_string(),
                    message: format!("Failed to decode status: {}", e),
                })?
        }
        None => {
            return Ok(ports::transaction::CommitArtifactFinalizeResult::Conflict);
        }
    };

    if outbox_status == "dead" {
        return Ok(ports::transaction::CommitArtifactFinalizeResult::ObsoleteBecauseProjectDeleted);
    }
    if outbox_status == "done" {
        return Ok(ports::transaction::CommitArtifactFinalizeResult::AlreadyFinalized);
    }
    if outbox_status != "processing" {
        return Ok(ports::transaction::CommitArtifactFinalizeResult::Conflict);
    }

    // 2. Check if project exists
    let project_exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?)")
            .bind(project_id.to_string())
            .fetch_one(&mut **tx)
            .await
            .map_err(|e| {
                crate::sqlite::helpers::map_sqlite_error("Failed to check project existence", e)
            })?;

    if !project_exists {
        return Ok(ports::transaction::CommitArtifactFinalizeResult::ObsoleteBecauseProjectDeleted);
    }

    // 3. Query artifact by id
    let artifact_row = sqlx::query(
        "SELECT project_id, state, location_kind, location_value FROM artifacts WHERE id = ?",
    )
    .bind(artifact_id.to_string())
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| crate::sqlite::helpers::map_sqlite_error("Failed to query artifact", e))?;

    let (art_project_id, state, location_kind, location_value) = match artifact_row {
        Some(row) => {
            use sqlx::Row;
            let pid: String =
                row.try_get("project_id")
                    .map_err(|e| PortError::InvalidStoredData {
                        entity_type: "artifact".to_string(),
                        entity_id: artifact_id.to_string(),
                        field: "project_id".to_string(),
                        message: format!("Failed to decode project_id: {}", e),
                    })?;
            let s: String = row
                .try_get("state")
                .map_err(|e| PortError::InvalidStoredData {
                    entity_type: "artifact".to_string(),
                    entity_id: artifact_id.to_string(),
                    field: "state".to_string(),
                    message: format!("Failed to decode state: {}", e),
                })?;
            let lk: String =
                row.try_get("location_kind")
                    .map_err(|e| PortError::InvalidStoredData {
                        entity_type: "artifact".to_string(),
                        entity_id: artifact_id.to_string(),
                        field: "location_kind".to_string(),
                        message: format!("Failed to decode location_kind: {}", e),
                    })?;
            let lv: String =
                row.try_get("location_value")
                    .map_err(|e| PortError::InvalidStoredData {
                        entity_type: "artifact".to_string(),
                        entity_id: artifact_id.to_string(),
                        field: "location_value".to_string(),
                        message: format!("Failed to decode location_value: {}", e),
                    })?;
            (pid, s, lk, lv)
        }
        None => {
            // Project exists, but artifact is missing
            return Ok(ports::transaction::CommitArtifactFinalizeResult::Conflict);
        }
    };

    if art_project_id != project_id.to_string() {
        return Ok(ports::transaction::CommitArtifactFinalizeResult::Conflict);
    }

    if state == "ready" {
        if location_kind == "StorageKey" && location_value == ready_key {
            return Ok(ports::transaction::CommitArtifactFinalizeResult::AlreadyFinalized);
        } else {
            return Ok(ports::transaction::CommitArtifactFinalizeResult::Conflict);
        }
    }

    if state != "pending_finalize" {
        return Ok(ports::transaction::CommitArtifactFinalizeResult::Conflict);
    }

    // CAS Update: pending_finalize -> ready
    let result = sqlx::query(
        r#"
        UPDATE artifacts
        SET state = 'ready',
            location_kind = 'StorageKey',
            location_value = ?,
            ready_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
            updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
        WHERE id = ? AND project_id = ? AND state = 'pending_finalize'
        "#,
    )
    .bind(ready_key)
    .bind(artifact_id.to_string())
    .bind(project_id.to_string())
    .execute(&mut **tx)
    .await
    .map_err(|e| {
        crate::sqlite::helpers::map_sqlite_error("Failed to update artifact to ready", e)
    })?;

    if result.rows_affected() != 1 {
        return Ok(ports::transaction::CommitArtifactFinalizeResult::Conflict);
    }

    Ok(ports::transaction::CommitArtifactFinalizeResult::Committed)
}
