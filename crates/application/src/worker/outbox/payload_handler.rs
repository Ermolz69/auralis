use domain::outbox::OutboxPayload;
use ports::artifact_index::ArtifactIndex;
use ports::events::AppEventPublisher;
use ports::storage::ArtifactStore;
use ports::transaction::StorageUnitOfWork;
use ports::workspace::TempWorkspacePort;
use std::sync::Arc;

use crate::error::ApplicationError;
use crate::services::artifact_finalizer::ArtifactFinalizer;

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
    pub async fn process_payload(&self, payload: &OutboxPayload) -> Result<(), ApplicationError> {
        match payload {
            OutboxPayload::FinalizeStagedArtifact {
                artifact_id,
                staging_key,
                final_key,
            } => {
                let finalizer = ArtifactFinalizer::new(
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
            OutboxPayload::DeleteWorkspaceFile { workspace_key } => {
                if let Err(e) = self.workspace_port.delete_allocation(workspace_key).await {
                    eprintln!(
                        "OutboxWorker: Failed to delete workspace allocation {}: {}",
                        workspace_key, e
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
                println!(
                    "Terminal lifecycle applied for project {}: {:?}",
                    project_id, res
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
