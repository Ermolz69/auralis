use std::sync::Arc;
use std::time::Duration;

use domain::outbox::OutboxPayload;
use ports::artifact_index::ArtifactIndex;
use ports::repository::OutboxRepository;
use ports::storage::ArtifactStore;

use ports::transaction::StorageUnitOfWork;

use crate::error::ApplicationError;

pub struct OutboxWorker<O, S, I, U>
where
    O: OutboxRepository,
    S: ArtifactStore,
    I: ArtifactIndex,
    U: StorageUnitOfWork,
{
    outbox_repo: O,
    artifact_store: S,
    artifact_index: I,
    uow: U,
    workspace_root: std::path::PathBuf,
}

impl<O, S, I, U> OutboxWorker<O, S, I, U>
where
    O: OutboxRepository,
    S: ArtifactStore + Clone,
    I: ArtifactIndex + Clone,
    U: StorageUnitOfWork,
{
    pub fn new(
        outbox_repo: O,
        artifact_store: S,
        artifact_index: I,
        uow: U,
        workspace_root: std::path::PathBuf,
    ) -> Self {
        Self {
            outbox_repo,
            artifact_store,
            artifact_index,
            uow,
            workspace_root,
        }
    }

    pub async fn process_pending_messages(&self, limit: usize) -> Result<usize, ApplicationError> {
        let messages = self.outbox_repo.fetch_pending(limit).await?;
        let count = messages.len();
        if count == 0 {
            return Ok(0);
        }

        let worker_id = uuid::Uuid::new_v4().to_string();

        for message in messages {
            // Lock the message
            let claimed = self
                .outbox_repo
                .mark_processing(&message.id, &worker_id)
                .await
                .unwrap_or_default();

            if !claimed {
                // Another worker took it or it was no longer pending
                continue;
            }

            let result = self.process_payload(&message.payload).await;

            match result {
                Ok(_) => {
                    let _ = self.outbox_repo.mark_done(&message.id).await;
                }
                Err(e) => {
                    let _ = self
                        .outbox_repo
                        .mark_failed(&message.id, &e.to_string())
                        .await;
                }
            }
        }

        Ok(count)
    }

    async fn process_payload(&self, payload: &OutboxPayload) -> Result<(), ApplicationError> {
        match payload {
            OutboxPayload::FinalizeStagedArtifact {
                artifact_id,
                staging_key,
                final_key,
            } => {
                let finalizer = crate::services::artifact_finalizer::ArtifactFinalizer::new(
                    self.artifact_index.clone(),
                    self.artifact_store.clone(),
                );

                if !finalizer
                    .finalize(artifact_id, staging_key, final_key)
                    .await?
                {
                    // If it returns false, it means project/artifact was deleted
                    return Ok(());
                }
            }
            OutboxPayload::DeleteStorageKey { storage_key } => {
                self.artifact_store.delete_storage_key(storage_key).await?;
            }
            OutboxPayload::DeleteProjectArtifactDir { project_id } => {
                self.artifact_store.delete_project_dir(project_id).await?;
            }
            OutboxPayload::DeleteTempPath { path } => {
                let target_path = std::path::Path::new(path);
                match tokio::fs::canonicalize(target_path).await {
                    Ok(canonical_target) => {
                        let canonical_workspace = tokio::fs::canonicalize(&self.workspace_root)
                            .await
                            .unwrap_or_else(|_| self.workspace_root.clone());
                        if canonical_target.starts_with(&canonical_workspace) {
                            let _ = tokio::fs::remove_file(canonical_target).await;
                        } else {
                            eprintln!(
                                "OutboxWorker: Refused to delete absolute path outside workspace: {}",
                                path
                            );
                            return Err(ApplicationError::InvalidOperation {
                                message: "Security violation: DeleteTempPath outside workspace"
                                    .to_string(),
                            });
                        }
                    }
                    Err(_) => {
                        // Missing file = success
                    }
                }
            }
            OutboxPayload::DeleteWorkspaceFile { workspace_key } => {
                match domain::outbox::models::resolve_workspace_file(
                    &self.workspace_root,
                    workspace_key,
                ) {
                    Ok(target_path) => {
                        match tokio::fs::canonicalize(&target_path).await {
                            Ok(canonical_target) => {
                                let canonical_workspace =
                                    tokio::fs::canonicalize(&self.workspace_root)
                                        .await
                                        .unwrap_or_else(|_| self.workspace_root.clone());
                                if canonical_target.starts_with(&canonical_workspace) {
                                    let _ = tokio::fs::remove_file(canonical_target).await;
                                } else {
                                    eprintln!(
                                        "OutboxWorker: Refused to delete resolved path outside workspace: {}",
                                        workspace_key
                                    );
                                    return Err(ApplicationError::InvalidOperation { message: "Security violation: DeleteWorkspaceFile outside workspace".to_string() });
                                }
                            }
                            Err(_) => {
                                // Missing file = success
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("OutboxWorker: Invalid workspace_key: {}", e);
                        return Err(ApplicationError::InvalidOperation {
                            message: "Validation error: invalid workspace key".to_string(),
                        });
                    }
                }
            }
            OutboxPayload::HandleTerminalJobState {
                job_id,
                project_id,
                outcome,
            } => {
                let command = ports::transaction::ApplyTerminalLifecycle {
                    project_id: project_id.clone(),
                    job_id: job_id.clone(),
                    outcome: outcome.clone(),
                };
                let res = self
                    .uow
                    .apply_terminal_lifecycle_conditionally(command)
                    .await?;
                // Even if IgnoredStale or AlreadyApplied, we consider it done for the outbox.
                println!(
                    "Terminal lifecycle applied for project {}: {:?}",
                    project_id, res
                );
            }
        }

        Ok(())
    }

    pub async fn run_loop(self: Arc<Self>, mut shutdown_rx: tokio::sync::mpsc::Receiver<()>) {
        let mut interval = tokio::time::interval(Duration::from_secs(5));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.process_pending_messages(10).await {
                        eprintln!("OutboxWorker error: {}", e);
                    }
                    // Run staging janitor
                    if let Err(e) = self.artifact_store.cleanup_stale_staging(Duration::from_secs(24 * 3600)).await {
                        eprintln!("OutboxWorker staging janitor error: {}", e);
                    }
                }
                _ = shutdown_rx.recv() => {
                    println!("OutboxWorker shutting down...");
                    break;
                }
            }
        }
    }
}
