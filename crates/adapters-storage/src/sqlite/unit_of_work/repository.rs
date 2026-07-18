use async_trait::async_trait;
use sqlx::SqlitePool;

use domain::outbox::{OutboxMessage, OutboxPayload};
use ports::error::PortError;
use ports::transaction::{
    CommitArtifactFinalize, CommitJobUpdate, CommitManagedSourceImport, CommitPipelineStart,
    CommitPipelineStartFailure, CommitProjectDelete, CommitProjectDeleteResult,
    CommitStagedArtifactWrite, CommitTranscriptImport, StorageUnitOfWork,
};

use super::artifact_writes::{finalize_artifact, save_artifact};
use super::job_writes::{insert_job, update_job};
use super::outbox_writes::save_outbox_message;
use super::project_writes::{delete_project, update_project};

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
    async fn commit_transcript_import(
        &self,
        command: CommitTranscriptImport,
    ) -> Result<(), PortError> {
        let mut tx = self.pool.begin().await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to begin transaction", e)
        })?;

        update_project(&mut tx, &command.project).await?;
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
                OutboxMessage::new(OutboxPayload::DeleteWorkspaceFile { workspace_key: key });
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
                OutboxMessage::new(OutboxPayload::DeleteWorkspaceFile { workspace_key: key });
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

        update_project(&mut tx, &command.project).await?;
        save_artifact(&mut tx, command.project.id(), &command.artifact).await?;

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
        let mut tx = self.pool.begin().await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to begin transaction", e)
        })?;

        let row = sqlx::query_as::<_, crate::sqlite::project_row::ProjectRow>(
            "SELECT * FROM projects WHERE id = ?",
        )
        .bind(command.project_id.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| crate::sqlite::helpers::map_sqlite_error("Failed to fetch project", e))?;

        let row = match row {
            Some(r) => r,
            None => {
                return Err(PortError::NotFound {
                    resource: format!("Project {}", command.project_id),
                });
            }
        };

        let mut project = crate::sqlite::project_mapper::row_to_project(row)?;

        let res = project
            .apply_terminal_transition(&command.job_id, command.outcome)
            .map_err(|e| PortError::Conflict {
                resource: "Project Transition".to_string(),
                message: e.to_string(),
            })?;

        if matches!(
            res,
            domain::project::status::TerminalTransitionResult::Applied { .. }
        ) {
            update_project(&mut tx, &project).await?;
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
        let mut tx = self.pool.begin().await.map_err(|e| {
            crate::sqlite::helpers::map_sqlite_error("Failed to begin transaction", e)
        })?;

        let result = finalize_artifact(
            &mut tx,
            &command.message_id.to_string(),
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
