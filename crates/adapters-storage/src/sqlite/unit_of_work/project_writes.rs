use super::outbox_writes::save_outbox_message;
use domain::outbox::{OutboxMessage, OutboxPayload};
use domain::project::Project;
use ports::error::PortError;
use sqlx::{Sqlite, Transaction};

use crate::sqlite::project_mapper::project_to_row_values;

pub(super) async fn update_project(
    tx: &mut Transaction<'_, Sqlite>,
    project: &Project,
) -> Result<(), PortError> {
    let row = project_to_row_values(project)?;
    let result = sqlx::query(
        r#"
        UPDATE projects SET
            title = ?,
            status = ?,
            source_json = ?,
            metadata_json = ?,
            source_language = ?,
            target_language = ?,
            transcript_json = ?,
            active_job_id = ?,
            last_terminal_job_id = ?,
            updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(row.title)
    .bind(row.status)
    .bind(row.source_json)
    .bind(row.metadata_json)
    .bind(row.source_language)
    .bind(row.target_language)
    .bind(row.transcript_json)
    .bind(row.active_job_id)
    .bind(row.last_terminal_job_id)
    .bind(row.updated_at)
    .bind(row.id)
    .execute(&mut **tx)
    .await
    .map_err(|e| crate::sqlite::helpers::map_sqlite_error("Failed to update project in tx", e))?;

    if result.rows_affected() == 0 {
        return Err(PortError::NotFound {
            resource: "Project".to_string(),
        });
    }

    Ok(())
}

pub(super) async fn delete_project(
    tx: &mut Transaction<'_, Sqlite>,
    project_id: &domain::project::ProjectId,
) -> Result<Vec<domain::job::JobId>, PortError> {
    // 1. Acquire write lock to serialize and check existence
    let exists = sqlx::query("UPDATE projects SET id = id WHERE id = ?")
        .bind(project_id.to_string())
        .execute(&mut **tx)
        .await
        .map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to verify project existence", e)
        })?;

    if exists.rows_affected() == 0 {
        return Err(PortError::NotFound {
            resource: format!("Project {}", project_id),
        });
    }

    // 2. Query Job IDs belonging to the project
    let job_records = sqlx::query("SELECT id FROM jobs WHERE project_id = ?")
        .bind(project_id.to_string())
        .fetch_all(&mut **tx)
        .await
        .map_err(|e| crate::sqlite::helpers::map_sqlite_error("Failed to fetch job IDs", e))?;

    let deleted_job_ids: Vec<domain::job::JobId> = job_records
        .into_iter()
        .map(|record| -> Result<domain::job::JobId, PortError> {
            use sqlx::Row;
            let id_str: String =
                record
                    .try_get("id")
                    .map_err(|e| PortError::InvalidStoredData {
                        entity_type: "job".to_string(),
                        entity_id: "unknown".to_string(),
                        field: "id".to_string(),
                        message: format!("Failed to decode job ID: {}", e),
                    })?;
            std::str::FromStr::from_str(&id_str).map_err(|e| PortError::InvalidStoredData {
                entity_type: "job".to_string(),
                entity_id: id_str,
                field: "id".to_string(),
                message: format!("Invalid UUID: {}", e),
            })
        })
        .collect::<Result<_, _>>()?;

    // 3. Query StorageKeys of artifacts (only location_kind = 'StorageKey')
    let artifact_records = sqlx::query(
        "SELECT id, location_value FROM artifacts WHERE project_id = ? AND location_kind = 'StorageKey' ORDER BY id ASC",
    )
    .bind(project_id.to_string())
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| crate::sqlite::helpers::map_sqlite_error("Failed to fetch artifacts", e))?;

    let mut storage_keys_to_delete = Vec::new();
    for record in artifact_records {
        use sqlx::Row;
        let artifact_id: String =
            record
                .try_get("id")
                .map_err(|e| PortError::InvalidStoredData {
                    entity_type: "artifact".to_string(),
                    entity_id: "unknown".to_string(),
                    field: "id".to_string(),
                    message: format!("Failed to decode artifact ID: {}", e),
                })?;

        let key_str: String =
            record
                .try_get("location_value")
                .map_err(|e| PortError::InvalidStoredData {
                    entity_type: "artifact".to_string(),
                    entity_id: artifact_id.clone(),
                    field: "location_value".to_string(),
                    message: format!("Failed to decode location_value: {}", e),
                })?;

        let storage_key =
            std::str::FromStr::from_str(&key_str).map_err(|e| PortError::InvalidStoredData {
                entity_type: "artifact".to_string(),
                entity_id: artifact_id,
                field: "location_value".to_string(),
                message: format!("Invalid StorageKey: {}", e),
            })?;

        storage_keys_to_delete.push(storage_key);
    }

    // 4. Cancel existing outbox messages for this project (done BEFORE creating new outbox messages)
    let now = domain::chrono::Utc::now();
    let timestamp = crate::sqlite::helpers::format_db_timestamp(now);
    let error_json = r#"{"reason": "obsolete due to project deletion"}"#;

    sqlx::query(
        "UPDATE outbox_messages SET status = 'dead', updated_at = ?, last_error = ? WHERE aggregate_type = 'project' AND aggregate_id = ? AND status IN ('pending', 'processing')"
    )
    .bind(timestamp)
    .bind(error_json)
    .bind(project_id.to_string())
    .execute(&mut **tx)
    .await
    .map_err(|e| {
        crate::sqlite::helpers::map_sqlite_error("Failed to cancel existing outbox messages", e)
    })?;

    // 5. Insert outbox DeleteStorageKey for each StorageKey artifact
    for storage_key in storage_keys_to_delete {
        let msg = OutboxMessage::new(OutboxPayload::DeleteStorageKey { storage_key });
        save_outbox_message(tx, &msg).await?;
    }

    // 6. Insert outbox DeleteProjectArtifactDir
    let del_msg = OutboxMessage::new(OutboxPayload::DeleteProjectArtifactDir {
        project_id: project_id.clone(),
    });
    save_outbox_message(tx, &del_msg).await?;

    // 7. Delete project (cascade constraints will delete jobs and artifacts)
    let delete_result = sqlx::query("DELETE FROM projects WHERE id = ?")
        .bind(project_id.to_string())
        .execute(&mut **tx)
        .await
        .map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to delete project in tx", e)
        })?;

    if delete_result.rows_affected() != 1 {
        return Err(PortError::Conflict {
            resource: format!("Project {}", project_id),
            message: "Project concurrently deleted or missing".to_string(),
        });
    }

    Ok(deleted_job_ids)
}
