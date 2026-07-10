use async_trait::async_trait;
use sqlx::{Sqlite, SqlitePool, Transaction};

use domain::job::Job;
use domain::media::Artifact;
use domain::outbox::{OutboxMessage, OutboxPayload};
use domain::project::Project;
use ports::error::PortError;
use ports::transaction::{
    CommitJobUpdate, CommitProjectDelete, CommitStagedArtifactWrite, CommitTranscriptImport,
    StorageUnitOfWork,
};

use super::artifact_index::mapper::artifact_to_row_values;
use super::job_mapper::job_to_row_values;
use super::project_mapper::project_to_row_values;

pub struct SqliteStorageUnitOfWork {
    pool: SqlitePool,
}

impl SqliteStorageUnitOfWork {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

async fn save_project(
    tx: &mut Transaction<'_, Sqlite>,
    project: &Project,
) -> Result<(), PortError> {
    let row = project_to_row_values(project)?;
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
    .execute(&mut **tx)
    .await
    .map_err(|e| PortError::Unexpected {
        message: format!("Failed to save project in tx: {}", e),
    })?;
    Ok(())
}

async fn save_artifact(
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
    .map_err(|e| PortError::Unexpected {
        message: format!("Failed to add artifact in tx: {}", e),
    })?;
    Ok(())
}

async fn save_job(tx: &mut Transaction<'_, Sqlite>, job: &Job) -> Result<(), PortError> {
    let row = job_to_row_values(job)?;

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
    .execute(&mut **tx)
    .await
    .map_err(|e| PortError::Unexpected {
        message: format!("Failed to save job in tx: {}", e),
    })?;
    Ok(())
}

async fn save_outbox_message(
    tx: &mut Transaction<'_, Sqlite>,
    msg: &OutboxMessage,
) -> Result<(), PortError> {
    let kind = msg.payload.clone();
    let kind_str = match &kind {
        OutboxPayload::FinalizeStagedArtifact { .. } => "finalize_staged_artifact",
        OutboxPayload::DeleteStorageKey { .. } => "delete_storage_key",
        OutboxPayload::DeleteProjectArtifactDir { .. } => "delete_project_artifact_dir",
        OutboxPayload::DeleteTempPath { .. } => "delete_temp_path",
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
    .bind(msg.locked_by.clone())
    .bind(msg.last_error.clone())
    .bind(msg.created_at.to_rfc3339())
    .bind(msg.updated_at.to_rfc3339())
    .execute(&mut **tx)
    .await
    .map_err(|e| PortError::Unexpected {
        message: format!("Failed to add outbox message in tx: {}", e),
    })?;
    Ok(())
}

#[async_trait]
impl StorageUnitOfWork for SqliteStorageUnitOfWork {
    async fn commit_transcript_import(
        &self,
        command: CommitTranscriptImport,
    ) -> Result<(), PortError> {
        let mut tx = self.pool.begin().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to begin transaction: {}", e),
        })?;

        save_project(&mut tx, &command.project).await?;
        save_artifact(&mut tx, command.project.id(), &command.artifact).await?;

        let finalize_msg = OutboxMessage::new(OutboxPayload::FinalizeStagedArtifact {
            artifact_id: command.artifact.id.clone(),
            staging_key: command.staging_key,
            final_key: command.final_key,
        });
        save_outbox_message(&mut tx, &finalize_msg).await?;

        if let Some(path) = command.temp_path_to_delete {
            let del_msg = OutboxMessage::new(OutboxPayload::DeleteTempPath {
                path: path.to_string_lossy().to_string(),
            });
            save_outbox_message(&mut tx, &del_msg).await?;
        }

        tx.commit().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to commit transaction: {}", e),
        })?;

        Ok(())
    }

    async fn commit_staged_artifact_write(
        &self,
        command: CommitStagedArtifactWrite,
    ) -> Result<(), PortError> {
        let mut tx = self.pool.begin().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to begin transaction: {}", e),
        })?;

        save_artifact(&mut tx, &command.project_id, &command.artifact).await?;

        let finalize_msg = OutboxMessage::new(OutboxPayload::FinalizeStagedArtifact {
            artifact_id: command.artifact.id.clone(),
            staging_key: command.staging_key,
            final_key: command.final_key,
        });
        save_outbox_message(&mut tx, &finalize_msg).await?;

        if let Some(path) = command.temp_path_to_delete {
            let del_msg = OutboxMessage::new(OutboxPayload::DeleteTempPath {
                path: path.to_string_lossy().to_string(),
            });
            save_outbox_message(&mut tx, &del_msg).await?;
        }

        tx.commit().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to commit transaction: {}", e),
        })?;

        Ok(())
    }

    async fn commit_project_delete(&self, command: CommitProjectDelete) -> Result<(), PortError> {
        let mut tx = self.pool.begin().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to begin transaction: {}", e),
        })?;

        // 1. Insert outbox DeleteStorageKey for each StorageKey artifact
        for artifact in command.artifacts {
            if let domain::media::ArtifactLocation::StorageKey(storage_key) = artifact.location {
                let msg = OutboxMessage::new(OutboxPayload::DeleteStorageKey { storage_key });
                save_outbox_message(&mut tx, &msg).await?;
            }
        }

        // 2. Insert outbox DeleteProjectArtifactDir
        let del_msg = OutboxMessage::new(OutboxPayload::DeleteProjectArtifactDir {
            project_id: command.project_id.clone(),
        });
        save_outbox_message(&mut tx, &del_msg).await?;

        // 3. Delete project (artifacts will be cascade deleted)
        sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(command.project_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| PortError::Unexpected {
                message: format!("Failed to delete project in tx: {}", e),
            })?;

        tx.commit().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to commit transaction: {}", e),
        })?;

        Ok(())
    }

    async fn commit_job_update(&self, command: CommitJobUpdate) -> Result<(), PortError> {
        let mut tx = self.pool.begin().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to begin transaction: {}", e),
        })?;

        save_job(&mut tx, &command.job).await?;

        tx.commit().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to commit transaction: {}", e),
        })?;

        Ok(())
    }
}
