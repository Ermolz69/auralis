use domain::outbox::OutboxPayload;
use ports::artifact_finalization::{ArtifactFinalizationLookup, FinalizationMetadata};
use ports::events::AppEventPublisher;
use ports::storage::ArtifactStore;
use ports::transaction::StorageUnitOfWork;
use ports::workspace::TempWorkspacePort;
use std::sync::Arc;

use crate::error::ApplicationError;

pub struct PayloadHandler<S, I, U>
where
    S: ArtifactStore + Clone,
    I: ArtifactFinalizationLookup + Clone,
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
    I: ArtifactFinalizationLookup + Clone,
    U: StorageUnitOfWork,
{
    #[tracing::instrument(skip_all, fields(operation_id = %message_id))]
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
                let metadata = self
                    .artifact_index
                    .get_for_finalization(project_id, artifact_id)
                    .await?;
                let FinalizationMetadata::Artifact(artifact) = metadata else {
                    return Ok(());
                };
                if artifact.id != *artifact_id
                    || artifact.location
                        != domain::media::ArtifactLocation::StorageKey(final_key.clone())
                    || !matches!(
                        artifact.state,
                        domain::media::ArtifactState::PendingFinalize
                            | domain::media::ArtifactState::Ready
                    )
                {
                    return Err(ports::error::PortError::Conflict {
                        resource: "Artifact".into(),
                        message: "Finalization metadata does not match the committed intent".into(),
                    }
                    .into());
                }
                self.artifact_store
                    .finalize_staged_artifact(staging_key, final_key, artifact.size_bytes)
                    .await?;

                // 2. Commit transaction with CAS
                let cmd = ports::transaction::CommitArtifactFinalize {
                    message_id: message_id.clone(),
                    project_id: project_id.clone(),
                    artifact_id: artifact_id.clone(),
                    ready_key: final_key.clone(),
                };

                let commit_result = self.uow.commit_artifact_finalize(cmd).await?;

                match commit_result {
                    ports::transaction::CommitArtifactFinalizeResult::Committed => {
                        // Success
                    }
                    ports::transaction::CommitArtifactFinalizeResult::AlreadyFinalized => {
                        // Success, was finalized by another retry
                    }
                    ports::transaction::CommitArtifactFinalizeResult::ObsoleteBecauseProjectDeleted => {
                        // Project deleted -> Delete the final_key
                        let _ = self.artifact_store.delete_storage_key(final_key).await;
                    }
                    ports::transaction::CommitArtifactFinalizeResult::Conflict => {
                        return Err(ApplicationError::InvalidOperation {
                            message: "Conflict during artifact finalization transaction".to_string(),
                        });
                    }
                }
            }
            OutboxPayload::DeleteStorageKey { storage_key } => {
                self.artifact_store.delete_storage_key(storage_key).await?;
            }
            OutboxPayload::DeleteProjectArtifactDir { project_id } => {
                self.artifact_store.delete_project_dir(project_id).await?;
            }
            OutboxPayload::DeleteWorkspaceAllocation { workspace_key } => {
                if let Err(e) = self.workspace_port.delete_allocation(workspace_key).await {
                    tracing::error!(
                        error = %common::observability::redaction::DiagnosticError {
                            kind: "WorkspaceAllocationDeletionFailed",
                            code: None,
                            retryable: false,
                        },
                        "OutboxWorker: Failed to delete workspace allocation"
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
                tracing::debug!(
                    project_id = %project_id,
                    job_id = %job_id,
                    result = ?res,
                    "terminal lifecycle evaluated"
                );

                if let domain::project::status::TerminalTransitionResult::Applied {
                    transcript_ready,
                } = res
                {
                    super::notifications::publish_terminal_notifications(
                        self.event_publisher.as_ref(),
                        &project_id.to_string(),
                        &job_id.to_string(),
                        transcript_ready,
                    )
                    .await;
                }
            }
        }

        Ok(())
    }
}
