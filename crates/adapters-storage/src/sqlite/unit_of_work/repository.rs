use async_trait::async_trait;
use sqlx::SqlitePool;

use domain::outbox::{OutboxMessage, OutboxPayload};
use ports::error::PortError;
use ports::transaction::{
    CommitArtifactFinalize, CommitJobUpdate, CommitManagedSourceImport, CommitPipelineStart,
    CommitPipelineStartFailure, CommitProjectDelete, CommitProjectDeleteResult,
    CommitStagedArtifactWrite, CommitTranscriptImport, StorageUnitOfWork,
};

use super::artifact_writes::{finalize_artifact, insert_new_artifact, save_artifact};
use super::job_writes::{insert_job, update_job};
use super::outbox_writes::save_outbox_message;
use super::project_writes::{delete_project, update_project, update_project_conditional};

pub struct SqliteStorageUnitOfWork {
    pool: SqlitePool,
}

impl SqliteStorageUnitOfWork {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StorageUnitOfWork for SqliteStorageUnitOfWork {
    async fn commit_youtube_import(
        &self,
        command: ports::transaction::CommitYoutubeImport,
    ) -> Result<(), PortError> {
        super::youtube_import::commit(&self.pool, command).await
    }
    async fn commit_transcript_import(
        &self,
        command: CommitTranscriptImport,
    ) -> Result<(), PortError> {
        let mut tx = self.pool.begin().await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to begin transaction", e)
        })?;

        update_project_conditional(
            &mut tx,
            &command.project,
            command.project.revision(),
            command.expected_project_updated_at,
            &command.expected_status,
            command.expected_active_job_id.as_ref(),
        )
        .await?;
        save_artifact(&mut tx, command.project.id(), &command.artifact).await?;

        let finalize_msg = OutboxMessage::new(OutboxPayload::FinalizeStagedArtifact {
            project_id: command.project.id().clone(),
            artifact_id: command.artifact.id.clone(),
            staging_key: command.staging_key,
            final_key: command.final_key,
        });
        save_outbox_message(&mut tx, &finalize_msg).await?;

        if let Some(key) = command.temp_workspace_key {
            let del_msg =
                OutboxMessage::new(OutboxPayload::DeleteWorkspaceAllocation { workspace_key: key })
                    .for_project(command.project.id());
            save_outbox_message(&mut tx, &del_msg).await?;
        }

        tx.commit().await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to commit transaction", e)
        })?;

        Ok(())
    }

    async fn commit_staged_artifact_write(
        &self,
        command: CommitStagedArtifactWrite,
    ) -> Result<(), PortError> {
        command.validate()?;

        let mut tx = self.pool.begin().await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to begin transaction", e)
        })?;

        save_artifact(&mut tx, &command.project_id, &command.artifact).await?;

        let finalize_msg = OutboxMessage::new(OutboxPayload::FinalizeStagedArtifact {
            project_id: command.project_id.clone(),
            artifact_id: command.artifact.id.clone(),
            staging_key: command.staging_key,
            final_key: command.final_key,
        });
        save_outbox_message(&mut tx, &finalize_msg).await?;

        if let Some(key) = command.temp_workspace_key {
            let del_msg =
                OutboxMessage::new(OutboxPayload::DeleteWorkspaceAllocation { workspace_key: key })
                    .for_project(&command.project_id);
            save_outbox_message(&mut tx, &del_msg).await?;
        }

        tx.commit().await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to commit transaction", e)
        })?;

        Ok(())
    }

    async fn commit_managed_source_import(
        &self,
        command: CommitManagedSourceImport,
    ) -> Result<(), PortError> {
        command.validate()?;

        let mut tx = self.pool.begin().await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to begin transaction", e)
        })?;

        update_project_conditional(
            &mut tx,
            &command.project,
            command.project.revision(),
            command.original_updated_at,
            &domain::project::ProjectStatus::Draft,
            None,
        )
        .await?;
        insert_new_artifact(&mut tx, command.project.id(), &command.artifact).await?;

        let finalize_msg = OutboxMessage::new(OutboxPayload::FinalizeStagedArtifact {
            project_id: command.project.id().clone(),
            artifact_id: command.artifact.id.clone(),
            staging_key: command.staging_key,
            final_key: command.final_key,
        });
        save_outbox_message(&mut tx, &finalize_msg).await?;

        tx.commit().await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to commit transaction", e)
        })?;

        Ok(())
    }

    async fn commit_project_delete(
        &self,
        command: CommitProjectDelete,
    ) -> Result<CommitProjectDeleteResult, PortError> {
        let mut tx = self.pool.begin().await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to acquire connection", e)
        })?;

        let deleted_job_ids = delete_project(&mut tx, &command.project_id).await?;

        tx.commit().await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to commit transaction", e)
        })?;

        Ok(CommitProjectDeleteResult { deleted_job_ids })
    }

    async fn commit_job_update(&self, command: CommitJobUpdate) -> Result<(), PortError> {
        let mut tx = self.pool.begin().await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to begin transaction", e)
        })?;

        update_job(&mut tx, &command.job, command.expected_revision).await?;

        tx.commit().await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to commit transaction", e)
        })?;

        Ok(())
    }

    async fn commit_pipeline_start(&self, command: CommitPipelineStart) -> Result<(), PortError> {
        command.validate()?;

        let mut tx = self.pool.begin().await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to begin transaction", e)
        })?;

        update_project(&mut tx, &command.project).await?;
        insert_job(&mut tx, &command.job).await?;

        tx.commit().await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to commit transaction", e)
        })?;

        Ok(())
    }

    async fn commit_pipeline_start_failure(
        &self,
        command: CommitPipelineStartFailure,
    ) -> Result<(), PortError> {
        command.validate()?;

        let mut tx = self.pool.begin().await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to begin transaction", e)
        })?;

        update_project(&mut tx, &command.project).await?;
        update_job(&mut tx, &command.job, command.expected_job_revision).await?;

        tx.commit().await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to commit transaction", e)
        })?;

        Ok(())
    }

    async fn commit_terminal_job_update(
        &self,
        command: ports::transaction::CommitTerminalJobUpdate,
    ) -> Result<(), PortError> {
        let mut tx = self.pool.begin().await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to begin transaction", e)
        })?;

        update_job(&mut tx, &command.job, command.expected_revision).await?;

        let mut outbox_msg = OutboxMessage::new(OutboxPayload::HandleTerminalJobState {
            job_id: command.job.id().clone(),
            project_id: command.project_id,
            outcome: command.outcome,
        });
        outbox_msg.deduplication_key = Some(command.deduplication_key);

        save_outbox_message(&mut tx, &outbox_msg).await?;

        tx.commit().await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to commit transaction", e)
        })?;

        Ok(())
    }

    async fn apply_terminal_lifecycle_conditionally(
        &self,
        command: ports::transaction::ApplyTerminalLifecycle,
    ) -> Result<domain::project::status::TerminalTransitionResult, PortError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to begin transaction", e)
        })?;

        if let Some((message_id, token)) = &command.attempt {
            let owned: i64 = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM outbox_messages WHERE id = ?
                 AND status = 'processing' AND locked_by = ?)",
            )
            .bind(message_id.to_string())
            .bind(token)
            .fetch_one(&mut *tx)
            .await
            .map_err(|error| {
                crate::sqlite::helpers::map_sqlite_error("Check terminal attempt", error)
            })?;
            if owned != 1 {
                return Err(PortError::Conflict {
                    resource: "Terminal outbox attempt".into(),
                    message: "The attempt no longer owns the terminal transition".into(),
                });
            }
        }

        let row = sqlx::query_as::<_, crate::sqlite::project_row::ProjectRow>(
            "SELECT * FROM projects WHERE id = ?",
        )
        .bind(command.project_id.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| crate::sqlite::helpers::map_sqlite_error("Failed to fetch project", e))?;

        let Some(row) = row else {
            return Ok(domain::project::status::TerminalTransitionResult::ProjectMissing);
        };

        let mut project = crate::sqlite::project_mapper::row_to_project(row)?;
        let expected_revision = project.revision();

        let res = project
            .apply_terminal_transition(&command.job_id, command.outcome.clone())
            .map_err(|e| PortError::Conflict {
                resource: "Project Transition".to_string(),
                message: e.to_string(),
            })?;

        if matches!(
            res,
            domain::project::status::TerminalTransitionResult::Applied { .. }
        ) {
            let values = crate::sqlite::project_mapper::project_to_row_values(&project)?;
            let terminal_job_status = match command.outcome {
                domain::job::TerminalOutcome::Completed => "completed",
                domain::job::TerminalOutcome::Failed => "failed",
                domain::job::TerminalOutcome::Cancelled => "cancelled",
            };
            let require_ready = matches!(command.outcome, domain::job::TerminalOutcome::Completed);
            let attempt_id = command.attempt.as_ref().map(|(id, _)| id.to_string());
            let attempt_token = command.attempt.as_ref().map(|(_, token)| token.as_str());
            let result = sqlx::query(
                "UPDATE projects SET revision = revision + 1, status = ?, active_job_id = NULL,
                 last_terminal_job_id = ?, updated_at = ?
                 WHERE id = ? AND revision = ? AND revision < 9223372036854775807
                   AND status = 'Processing' AND active_job_id = ?
                   AND (? IS NULL OR EXISTS (
                     SELECT 1 FROM outbox_messages o JOIN jobs j ON j.id = ?
                     WHERE o.id = ? AND o.status = 'processing' AND o.locked_by = ?
                       AND j.project_id = projects.id AND j.status = ?))
                   AND (? = 0 OR NOT EXISTS (
                     SELECT 1 FROM artifacts a WHERE a.project_id = projects.id
                       AND a.state = 'pending_finalize'))",
            )
            .bind(values.status)
            .bind(command.job_id.to_string())
            .bind(values.updated_at)
            .bind(command.project_id.to_string())
            .bind(expected_revision as i64)
            .bind(command.job_id.to_string())
            .bind(attempt_id.as_deref())
            .bind(command.job_id.to_string())
            .bind(attempt_id.as_deref())
            .bind(attempt_token)
            .bind(terminal_job_status)
            .bind(require_ready)
            .execute(&mut *tx)
            .await
            .map_err(|error| {
                crate::sqlite::helpers::map_sqlite_error("Apply terminal project state", error)
            })?;
            if result.rows_affected() != 1 {
                return Err(PortError::Conflict {
                    resource: "Project terminal transition".into(),
                    message: "Revision, job, attempt, or publication prerequisites changed".into(),
                });
            }
        }

        tx.commit().await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to commit transaction", e)
        })?;

        Ok(res)
    }

    async fn commit_artifact_finalize(
        &self,
        command: CommitArtifactFinalize,
    ) -> Result<ports::transaction::CommitArtifactFinalizeResult, PortError> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to begin transaction", e)
        })?;

        let result = finalize_artifact(
            &mut tx,
            &command.message_id.to_string(),
            command.claim_token.as_deref(),
            &command.project_id,
            &command.artifact_id,
            &command.ready_key,
        )
        .await?;

        if matches!(
            result,
            ports::transaction::CommitArtifactFinalizeResult::Conflict
        ) {
            tx.rollback().await.ok();
            return Ok(result);
        }

        tx.commit().await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to commit finalize transaction", e)
        })?;

        Ok(result)
    }
}
