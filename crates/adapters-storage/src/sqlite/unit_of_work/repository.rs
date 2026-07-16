use async_trait::async_trait;
use sqlx::SqlitePool;

use domain::outbox::{OutboxMessage, OutboxPayload};
use ports::error::PortError;
use ports::transaction::{
    CommitJobUpdate, CommitManagedSourceImport, CommitPipelineStart, CommitPipelineStartFailure,
    CommitProjectDelete, CommitProjectDeleteResult, CommitStagedArtifactWrite,
    CommitTranscriptImport, StorageUnitOfWork,
};

use super::artifact_writes::save_artifact;
use super::job_writes::{insert_job, update_job};
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

    async fn commit_project_delete(
        &self,
        command: CommitProjectDelete,
    ) -> Result<CommitProjectDeleteResult, PortError> {
        // We'll acquire a write lock immediately
        // by executing a dummy update to ensure equivalent serialization (IMMEDIATE transaction semantics).
        let mut tx = self.pool.begin().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to acquire connection: {}", e),
        })?;

        // 1. Acquire write lock to serialize and prevent concurrent reads from missing this delete
        // and to check existence.
        let exists = sqlx::query("UPDATE projects SET id = id WHERE id = ?")
            .bind(command.project_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| PortError::Unexpected {
                message: format!("Failed to verify project existence: {}", e),
            })?;

        if exists.rows_affected() == 0 {
            return Err(PortError::NotFound {
                resource: format!("Project {}", command.project_id),
            });
        }

        // 2. Query Job IDs belonging to the project
        let job_records = sqlx::query("SELECT id FROM jobs WHERE project_id = ?")
            .bind(command.project_id.to_string())
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| PortError::Unexpected {
                message: format!("Failed to fetch job IDs: {}", e),
            })?;

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

        // 3. Query StorageKeys of artifacts for the project
        let artifact_records = sqlx::query(
            "SELECT id, storage_key FROM artifacts WHERE project_id = ? AND storage_key IS NOT NULL",
        )
        .bind(command.project_id.to_string())
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| PortError::Unexpected {
            message: format!("Failed to fetch artifacts: {}", e),
        })?;

        // 4. Insert outbox DeleteStorageKey for each StorageKey artifact
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
                    .try_get("storage_key")
                    .map_err(|e| PortError::InvalidStoredData {
                        entity_type: "artifact".to_string(),
                        entity_id: artifact_id.clone(),
                        field: "storage_key".to_string(),
                        message: format!("Failed to decode storage key: {}", e),
                    })?;

            let storage_key = std::str::FromStr::from_str(&key_str).map_err(|e| {
                PortError::InvalidStoredData {
                    entity_type: "artifact".to_string(),
                    entity_id: artifact_id,
                    field: "storage_key".to_string(),
                    message: format!("Invalid StorageKey: {}", e),
                }
            })?;

            let msg = OutboxMessage::new(OutboxPayload::DeleteStorageKey { storage_key });
            save_outbox_message(&mut tx, &msg).await?;
        }

        // 5. Insert outbox DeleteProjectArtifactDir
        let del_msg = OutboxMessage::new(OutboxPayload::DeleteProjectArtifactDir {
            project_id: command.project_id.clone(),
        });
        save_outbox_message(&mut tx, &del_msg).await?;

        // 6. Delete project (artifacts and jobs will be cascade deleted)
        let delete_result = sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(command.project_id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|e| PortError::Unexpected {
                message: format!("Failed to delete project in tx: {}", e),
            })?;

        if delete_result.rows_affected() != 1 {
            return Err(PortError::Conflict {
                resource: format!("Project {}", command.project_id),
                message: "Project concurrently deleted or missing".to_string(),
            });
        }

        tx.commit().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to commit transaction: {}", e),
        })?;

        Ok(CommitProjectDeleteResult { deleted_job_ids })
    }

    async fn commit_job_update(&self, command: CommitJobUpdate) -> Result<(), PortError> {
        let mut tx = self.pool.begin().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to begin transaction: {}", e),
        })?;

        update_job(&mut tx, &command.job).await?;

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

        let mut outbox_msg = OutboxMessage::new(OutboxPayload::HandleTerminalJobState {
            job_id: command.job.id().clone(),
            project_id: command.project_id,
            outcome: command.outcome,
        });
        outbox_msg.deduplication_key = Some(command.deduplication_key);

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

        if matches!(
            res,
            domain::project::status::TerminalTransitionResult::Applied { .. }
        ) {
            update_project(&mut tx, &project).await?;
        }

        tx.commit().await.map_err(|e| PortError::Unexpected {
            message: format!("Failed to commit transaction: {}", e),
        })?;

        Ok(res)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    use domain::media::{Artifact, ArtifactId};
    use domain::project::Project;
    use sqlx::SqlitePool;

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_commit_managed_source_import_writes_atomically() {
        let pool = setup_db().await;
        let uow = SqliteStorageUnitOfWork::new(pool.clone());

        let artifact = Artifact {
            id: ArtifactId::new(),
            kind: domain::media::ArtifactKind::SourceVideo,
            location: domain::media::ArtifactLocation::LocalPath("fake_path".into()),
            size_bytes: Some(1024),
            state: domain::media::ArtifactState::PendingFinalize,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            ready_at: None,
        };

        let mut project = Project::new("Tx Test".to_string());
        sqlx::query("INSERT INTO projects (id, title, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?)")
            .bind(project.id().to_string())
            .bind(project.title())
            .bind("Draft")
            .bind(project.created_at().to_rfc3339())
            .bind(project.updated_at().to_rfc3339())
            .execute(&pool)
            .await
            .unwrap();

        project
            .import_source(
                domain::media::MediaSource::ManagedLocalFile {
                    artifact_id: artifact.id.clone(),
                    original_filename: "test".into(),
                },
                None,
            )
            .unwrap();

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

    #[tokio::test]
    async fn test_commit_project_delete_rolls_back_on_invalid_job_id() {
        let pool = setup_db().await;
        let uow = SqliteStorageUnitOfWork::new(pool.clone());
        let project_id = domain::project::ProjectId::new();

        sqlx::query("INSERT INTO projects (id, title, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?)")
            .bind(project_id.to_string())
            .bind("Corrupt Job Test")
            .bind("Draft")
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(chrono::Utc::now().to_rfc3339())
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO jobs (id, project_id, kind, title, status, progress_json, error_json, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind("invalid-job-uuid-12345") // CORRUPTED UUID
            .bind(project_id.to_string())
            .bind("Extracting")
            .bind("Job title")
            .bind("Pending")
            .bind(r#"{}"#)
            .bind::<Option<String>>(None)
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(chrono::Utc::now().to_rfc3339())
            .execute(&pool)
            .await
            .unwrap();

        let cmd = CommitProjectDelete {
            project_id: project_id.clone(),
        };

        let result = uow.commit_project_delete(cmd).await;

        match result {
            Err(PortError::InvalidStoredData { field, .. }) => {
                assert_eq!(field, "id");
            }
            Ok(_) => panic!("Expected InvalidStoredData error, got Ok"),
            Err(e) => panic!("Expected InvalidStoredData error, got Err({:?})", e),
        }

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE id = ?")
            .bind(project_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);

        let outbox_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM outbox_messages")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(outbox_count, 0);
    }

    #[tokio::test]
    async fn test_commit_project_delete_conflict_when_missing() {
        let pool = setup_db().await;
        let uow = SqliteStorageUnitOfWork::new(pool.clone());
        let project_id = domain::project::ProjectId::new();

        let cmd = CommitProjectDelete {
            project_id: project_id.clone(),
        };

        let result = uow.commit_project_delete(cmd).await;

        match result {
            Err(PortError::NotFound { .. }) => {}
            Ok(_) => panic!("Expected NotFound on missing project initial check, got Ok"),
            Err(e) => panic!(
                "Expected NotFound on missing project initial check, got Err({:?})",
                e
            ),
        }
    }
}
