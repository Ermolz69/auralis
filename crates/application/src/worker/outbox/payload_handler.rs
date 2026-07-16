use domain::outbox::OutboxPayload;
use ports::artifact_index::ArtifactIndex;
use ports::events::AppEventPublisher;
use ports::storage::ArtifactStore;
use ports::transaction::StorageUnitOfWork;
use ports::workspace::TempWorkspacePort;
use std::sync::Arc;

use crate::error::ApplicationError;

pub struct PayloadHandler<S, I, U>
where
    S: ArtifactStore + Clone,
    I: ArtifactIndex + Clone,
    U: StorageUnitOfWork,
{
    pub artifact_store: S,
    pub artifact_index: I,
    pub uow: U,
    pub event_publisher: Arc<dyn AppEventPublisher>,
    pub workspace_port: Arc<dyn TempWorkspacePort>,
}

impl<S, I, U> PayloadHandler<S, I, U>
where
    S: ArtifactStore + Clone,
    I: ArtifactIndex + Clone,
    U: StorageUnitOfWork,
{
    pub async fn process_payload(
        &self,
        message_id: &domain::outbox::OutboxMessageId,
        payload: &OutboxPayload,
    ) -> Result<(), ApplicationError> {
        match payload {
            OutboxPayload::FinalizeStagedArtifact {
                project_id,
                artifact_id,
                staging_key,
                final_key,
            } => {
                // 1. Perform persistent move
                match self
                    .artifact_store
                    .finalize_staged_artifact(staging_key, final_key)
                    .await
                {
                    Ok(_) => {}
                    Err(ports::error::PortError::NotFound { .. }) => {
                        // Staging file missing. We must assume it was already finalized.
                    }
                    Err(e) => return Err(e.into()),
                }

                // 2. Commit transaction with CAS
                let cmd = ports::transaction::CommitArtifactFinalize {
                    message_id: message_id.clone(),
                    project_id: project_id.clone(),
                    artifact_id: artifact_id.clone(),
                    ready_key: final_key.clone(),
                };

                let commit_result = self.uow.commit_artifact_finalize(cmd).await;

                if let Err(ports::error::PortError::Conflict { .. }) = commit_result {
                    // Conflict means rows_affected == 0.
                    // The project was deleted, or this was concurrently finalized, or cancelled.
                    // We must compensate by deleting the final key since the DB rejected it.
                    let _ = self.artifact_store.delete_storage_key(final_key).await;
                    return Ok(());
                }

                commit_result?;
            }
            OutboxPayload::DeleteStorageKey { storage_key } => {
                self.artifact_store.delete_storage_key(storage_key).await?;
            }
            OutboxPayload::DeleteProjectArtifactDir { project_id } => {
                self.artifact_store.delete_project_dir(project_id).await?;
            }
            OutboxPayload::DeleteWorkspaceFile { workspace_key } => {
                if let Err(e) = self.workspace_port.delete_allocation(workspace_key).await {
                    tracing::error!(
                        workspace_key = %workspace_key,
                        "OutboxWorker: Failed to delete workspace allocation: {}",
                        e
                    );
                    return Err(ApplicationError::InvalidOperation {
                        message: format!("Failed to delete workspace allocation: {}", e),
                    });
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
                tracing::info!(
                    project_id = %project_id,
                    job_id = %job_id,
                    "Terminal lifecycle applied: {:?}",
                    res
                );

                if let domain::project::status::TerminalTransitionResult::Applied {
                    transcript_ready,
                } = res
                {
                    if transcript_ready {
                        let _ = self
                            .event_publisher
                            .publish_transcript_ready(&project_id.to_string(), &job_id.to_string())
                            .await;
                    }
                    let _ = self
                        .event_publisher
                        .publish_project_updated(&project_id.to_string())
                        .await;
                }
            }
        }

        Ok(())
    }
}
