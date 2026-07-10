use async_trait::async_trait;
use sqlx::SqlitePool;

use domain::outbox::{OutboxMessage, OutboxPayload};
use ports::error::PortError;
use ports::transaction::{
    CommitJobUpdate, CommitProjectDelete, CommitStagedArtifactWrite, CommitTranscriptImport,
    StorageUnitOfWork,
};

use super::artifact_writes::save_artifact;
use super::job_writes::save_job;
use super::outbox_writes::save_outbox_message;
use super::project_writes::save_project;

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
