use async_trait::async_trait;
use sqlx::SqlitePool;

use ports::error::PortError;
use ports::transaction::{TransactionGateway, UnitOfWorkData};

use super::artifact_mapper::artifact_to_row_values;

pub struct SqliteTransactionGateway {
    pool: SqlitePool,
}

impl SqliteTransactionGateway {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TransactionGateway for SqliteTransactionGateway {
    async fn execute(&self, data: UnitOfWorkData) -> Result<(), PortError> {
        let mut tx = self.pool.begin().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to begin transaction: {}", e),
        })?;

        // Process jobs_to_save
        for job in data.jobs_to_save {
            let row = super::job_mapper::job_to_row_values(&job)?;

            sqlx::query(
                r#"
                INSERT INTO jobs (
                    id, project_id, title, kind, status, stage, progress_json, error_json, 
                    created_at, updated_at, started_at, finished_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(id) DO UPDATE SET
                    title = excluded.title,
                    status = excluded.status,
                    stage = excluded.stage,
                    progress_json = excluded.progress_json,
                    error_json = excluded.error_json,
                    updated_at = excluded.updated_at,
                    started_at = excluded.started_at,
                    finished_at = excluded.finished_at
                "#,
            )
            .bind(row.id)
            .bind(row.project_id)
            .bind(row.title)
            .bind(row.kind)
            .bind(row.status)
            .bind(row.stage)
            .bind(row.progress_json)
            .bind(row.error_json)
            .bind(row.created_at)
            .bind(row.updated_at)
            .bind(row.started_at)
            .bind(row.finished_at)
            .execute(&mut *tx)
            .await
            .map_err(|e| PortError::Unexpected {
                message: format!("Failed to save job in tx: {}", e),
            })?;
        }

        // Process projects_to_save
        for project in data.projects_to_save {
            let row = super::project_mapper::project_to_row_values(&project)?;
            sqlx::query(
                r#"
                INSERT INTO projects (
                    id, title, status, source_json, metadata_json, 
                    source_language, target_language, transcript_json, 
                    created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(id) DO UPDATE SET
                    title = excluded.title,
                    status = excluded.status,
                    source_json = excluded.source_json,
                    metadata_json = excluded.metadata_json,
                    source_language = excluded.source_language,
                    target_language = excluded.target_language,
                    transcript_json = excluded.transcript_json,
                    updated_at = excluded.updated_at
                "#,
            )
            .bind(row.id)
            .bind(row.title)
            .bind(row.status)
            .bind(row.source_json)
            .bind(row.metadata_json)
            .bind(row.source_language)
            .bind(row.target_language)
            .bind(row.transcript_json)
            .bind(row.created_at)
            .bind(row.updated_at)
            .execute(&mut *tx)
            .await
            .map_err(|e| PortError::Unexpected {
                message: format!("Failed to save project in tx: {}", e),
            })?;
        }

        // Process artifacts_to_add
        for (project_id, artifact) in data.artifacts_to_add {
            let values = artifact_to_row_values(&project_id, &artifact)?;
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
            .execute(&mut *tx)
            .await
            .map_err(|e| PortError::Unexpected {
                message: format!("Failed to add artifact in tx: {}", e),
            })?;
        }

        // Process artifacts_to_delete
        for artifact_id in data.artifacts_to_delete {
            sqlx::query("DELETE FROM artifacts WHERE id = ?")
                .bind(artifact_id.to_string())
                .execute(&mut *tx)
                .await
                .map_err(|e| PortError::Unexpected {
                    message: format!("Failed to delete artifact in tx: {}", e),
                })?;
        }

        // Process outbox_messages
        for msg in data.outbox_messages {
            let kind = msg.payload.clone();
            let kind_str = match &kind {
                domain::outbox::OutboxPayload::FinalizeStagedArtifact { .. } => {
                    "finalize_staged_artifact"
                }
                domain::outbox::OutboxPayload::DeleteStorageKey { .. } => "delete_storage_key",
                domain::outbox::OutboxPayload::DeleteProjectArtifactDir { .. } => {
                    "delete_project_artifact_dir"
                }
                domain::outbox::OutboxPayload::DeleteTempPath { .. } => "delete_temp_path",
            };

            let payload_json =
                serde_json::to_string(&msg.payload).map_err(|e| PortError::Unexpected {
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
                INSERT INTO outbox_messages (
                    id, kind, payload_json, status, attempts, next_attempt_at,
                    locked_at, locked_by, last_error, created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(msg.id.to_string())
            .bind(kind_str)
            .bind(payload_json)
            .bind(status_str)
            .bind(msg.attempts)
            .bind(msg.next_attempt_at.to_rfc3339())
            .bind(msg.locked_at.map(|dt| dt.to_rfc3339()))
            .bind(msg.locked_by)
            .bind(msg.last_error)
            .bind(msg.created_at.to_rfc3339())
            .bind(msg.updated_at.to_rfc3339())
            .execute(&mut *tx)
            .await
            .map_err(|e| PortError::Unexpected {
                message: format!("Failed to add outbox message in tx: {}", e),
            })?;
        }

        // Process projects_to_delete
        for project_id in data.projects_to_delete {
            sqlx::query("DELETE FROM projects WHERE id = ?")
                .bind(project_id.to_string())
                .execute(&mut *tx)
                .await
                .map_err(|e| PortError::Unexpected {
                    message: format!("Failed to delete project in tx: {}", e),
                })?;
        }

        tx.commit().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to commit transaction: {}", e),
        })?;

        Ok(())
    }
}
