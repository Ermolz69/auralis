use async_trait::async_trait;
use sqlx::SqlitePool;

use domain::outbox::{OutboxMessage, OutboxPayload};
use ports::error::PortError;
use ports::transaction::{
    CommitJobUpdate, CommitManagedSourceImport, CommitPipelineStart, CommitPipelineStartFailure,
    CommitProjectDelete, CommitStagedArtifactWrite, CommitTranscriptImport, StorageUnitOfWork,
};

use super::artifact_writes::save_artifact;
use super::job_writes::{insert_job, save_job, update_job};
use super::outbox_writes::save_outbox_message;
use super::project_writes::update_project;

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
        let mut tx = self.pool.begin().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to begin transaction: {}", e),
        })?;

        update_project(&mut tx, &command.project).await?;
        save_artifact(&mut tx, command.project.id(), &command.artifact).await?;

        let finalize_msg = OutboxMessage::new(OutboxPayload::FinalizeStagedArtifact {
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

        if let Some(key) = command.temp_workspace_key {
            let del_msg =
                OutboxMessage::new(OutboxPayload::DeleteWorkspaceFile { workspace_key: key });
            save_outbox_message(&mut tx, &del_msg).await?;
        }

        tx.commit().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to commit transaction: {}", e),
        })?;

        Ok(())
    }

    async fn commit_managed_source_import(
        &self,
        command: CommitManagedSourceImport,
    ) -> Result<(), PortError> {
        command.validate()?;

        let mut tx = self.pool.begin().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to begin transaction: {}", e),
        })?;

        update_project(&mut tx, &command.project).await?;
        save_artifact(&mut tx, command.project.id(), &command.artifact).await?;

        let finalize_msg = OutboxMessage::new(OutboxPayload::FinalizeStagedArtifact {
            artifact_id: command.artifact.id.clone(),
            staging_key: command.staging_key,
            final_key: command.final_key,
        });
        save_outbox_message(&mut tx, &finalize_msg).await?;

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

    async fn commit_pipeline_start(&self, command: CommitPipelineStart) -> Result<(), PortError> {
        command.validate()?;

        let mut tx = self.pool.begin().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to begin transaction: {}", e),
        })?;

        update_project(&mut tx, &command.project).await?;
        insert_job(&mut tx, &command.job).await?;

        tx.commit().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to commit transaction: {}", e),
        })?;

        Ok(())
    }

    async fn commit_pipeline_start_failure(
        &self,
        command: CommitPipelineStartFailure,
    ) -> Result<(), PortError> {
        command.validate()?;

        let mut tx = self.pool.begin().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to begin transaction: {}", e),
        })?;

        update_project(&mut tx, &command.project).await?;
        update_job(&mut tx, &command.job).await?;

        tx.commit().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to commit transaction: {}", e),
        })?;

        Ok(())
    }

    async fn commit_terminal_job_update(
        &self,
        command: ports::transaction::CommitTerminalJobUpdate,
    ) -> Result<(), PortError> {
        let mut tx = self.pool.begin().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to begin transaction: {}", e),
        })?;

        update_job(&mut tx, &command.job).await?;

        use std::str::FromStr;
        let outbox_msg = OutboxMessage {
            id: domain::outbox::OutboxMessageId::from_str(&command.outbox_message_id).map_err(
                |e| PortError::Unexpected {
                    message: e.to_string(),
                },
            )?,
            payload: OutboxPayload::HandleTerminalJobState {
                job_id: command.job.id().clone(),
                project_id: command.project_id,
                outcome: command.outcome,
            },
            status: domain::outbox::OutboxMessageStatus::Pending,
            attempts: 0,
            next_attempt_at: chrono::Utc::now(),
            locked_at: None,
            locked_by: None,
            last_error: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        save_outbox_message(&mut tx, &outbox_msg).await?;

        tx.commit().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to commit transaction: {}", e),
        })?;

        Ok(())
    }

    async fn apply_terminal_lifecycle_conditionally(
        &self,
        command: ports::transaction::ApplyTerminalLifecycle,
    ) -> Result<domain::project::status::TerminalTransitionResult, PortError> {
        let mut tx = self.pool.begin().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to begin transaction: {}", e),
        })?;

        let row = sqlx::query_as::<_, crate::sqlite::project_row::ProjectRow>(
            "SELECT * FROM projects WHERE id = ?",
        )
        .bind(command.project_id.to_string())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to fetch project: {}", e),
        })?;

        let row = match row {
            Some(r) => r,
            None => {
                return Err(PortError::Unexpected {
                    message: "Project not found".to_string(),
                });
            }
        };

        let mut project = crate::sqlite::project_mapper::row_to_project(row)?;

        let res = project
            .apply_terminal_transition(&command.job_id, command.outcome)
            .map_err(|e| PortError::Unexpected {
                message: e.to_string(),
            })?;

        if res == domain::project::status::TerminalTransitionResult::Applied {
            update_project(&mut tx, &project).await?;
        }

        tx.commit().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to commit transaction: {}", e),
        })?;

        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::job::JobId;
    use domain::media::{Artifact, ArtifactId};
    use domain::project::Project;
    use sqlx::SqlitePool;

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./src/sqlite/migrations_runtime")
            .run(&pool)
            .await
            .unwrap();
        pool
    }

    #[tokio::test]
    async fn test_commit_managed_source_import_writes_atomically() {
        let pool = setup_db().await;
        let uow = SqliteStorageUnitOfWork::new(pool.clone());

        let mut project = Project::new("Tx Test".to_string());

        // 1. Pre-insert the project since update_project expects it to exist, wait actually update_project uses UPSERT?
        // Let's look at project_writes::update_project, it uses `ON CONFLICT(id) DO UPDATE`. So it works for insert too.

        let artifact = Artifact {
            id: ArtifactId::new(),
            kind: domain::media::ArtifactKind::OriginalSubtitle,
            location: domain::media::ArtifactLocation::LocalPath("fake_path".into()),
            size_bytes: Some(1024),
            state: domain::media::ArtifactState::PendingFinalize,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            ready_at: None,
        };

        let cmd = CommitManagedSourceImport {
            project: project.clone(),
            artifact: artifact.clone(),
            staging_key: "staging_key".to_string(),
            final_key: "final_key".to_string(),
        };

        uow.commit_managed_source_import(cmd).await.unwrap();

        // Verify project is in DB
        let project_row: Option<crate::sqlite::project_row::ProjectRow> =
            sqlx::query_as("SELECT * FROM projects WHERE id = ?")
                .bind(project.id().to_string())
                .fetch_optional(&pool)
                .await
                .unwrap();
        assert!(project_row.is_some());

        // Verify artifact is in DB
        let artifact_row: Option<crate::sqlite::artifact_index::row::ArtifactRow> =
            sqlx::query_as("SELECT * FROM artifacts WHERE id = ?")
                .bind(artifact.id.to_string())
                .fetch_optional(&pool)
                .await
                .unwrap();
        assert!(artifact_row.is_some());

        // Verify outbox message is in DB
        let outbox_rows: Vec<crate::sqlite::outbox_row::OutboxRow> =
            sqlx::query_as("SELECT * FROM outbox_messages")
                .fetch_all(&pool)
                .await
                .unwrap();

        assert_eq!(outbox_rows.len(), 1);
        let payload: OutboxPayload = serde_json::from_str(&outbox_rows[0].payload_json).unwrap();

        match payload {
            OutboxPayload::FinalizeStagedArtifact {
                artifact_id,
                staging_key,
                final_key,
            } => {
                assert_eq!(artifact_id, artifact.id);
                assert_eq!(staging_key, "staging_key");
                assert_eq!(final_key, "final_key");
            }
            _ => panic!("Expected FinalizeStagedArtifact payload"),
        }
    }
}
