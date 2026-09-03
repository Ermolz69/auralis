use std::sync::Arc;

use ports::artifact_index::ArtifactIndex;
use ports::events::AppEventPublisher;
use ports::repository::OutboxRepository;
use ports::storage::ArtifactStore;
use ports::transaction::StorageUnitOfWork;
use ports::workspace::TempWorkspacePort;

use super::payload_handler::PayloadHandler;
use super::report::OutboxBatchReport;
use crate::error::ApplicationError;

use super::maintenance::OutboxMaintenanceConfig;

pub struct OutboxWorker<O, S, I, U>
where
    O: OutboxRepository,
    S: ArtifactStore + Clone,
    I: ArtifactIndex + Clone,
    U: StorageUnitOfWork,
{
    pub(super) outbox_repo: O,
    pub(super) handler: PayloadHandler<S, I, U>,
    pub(super) config: OutboxMaintenanceConfig,
    pub(super) imports: Option<Arc<dyn ports::youtube_import::YoutubeImportJournal>>,
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
        event_publisher: Arc<dyn AppEventPublisher>,
        workspace_port: Arc<dyn TempWorkspacePort>,
        config: OutboxMaintenanceConfig,
    ) -> Self {
        let handler = PayloadHandler {
            artifact_store: artifact_store.clone(),
            artifact_index,
            uow,
            event_publisher,
            workspace_port,
        };

        Self {
            outbox_repo,
            handler,
            config,
            imports: None,
        }
    }

    pub fn with_imports(
        mut self,
        imports: Arc<dyn ports::youtube_import::YoutubeImportJournal>,
    ) -> Self {
        self.imports = Some(imports);
        self
    }

    pub fn process_pending_messages<'a>(
        &'a self,
        limit: usize,
    ) -> impl std::future::Future<Output = Result<OutboxBatchReport, ApplicationError>> + 'a {
        let span = tracing::info_span!("outbox_batch", action = "process_pending_messages");
        let mut guard = crate::observability::execution_summary::ExecutionSummaryGuard::new(
            span.clone(),
            crate::observability::execution_summary::OperationSummary::OutboxBatch {
                action: "process_pending_messages",
                status: "aborted".to_string(),
                failed_count: 0,
            },
        );

        async move {
            let res = self.process_pending_messages_inner(limit).await;
            match &res {
                Ok(report) => {
                    let failed_count = report.corrupted + report.storage_errors + report.dead;
                    let status = if failed_count > 0 {
                        "completed_with_errors"
                    } else {
                        "completed"
                    };
                    guard.update_summary(
                        crate::observability::execution_summary::OperationSummary::OutboxBatch {
                            action: "process_pending_messages",
                            status: status.to_string(),
                            failed_count: failed_count as u64,
                        },
                    );
                }
                Err(_) => guard.summary.update_status("failed"),
            }
            res
        }
    }

    async fn process_pending_messages_inner(
        &self,
        limit: usize,
    ) -> Result<OutboxBatchReport, ApplicationError> {
        let fetch_result = self.outbox_repo.fetch_pending(limit).await?;
        let messages = fetch_result.messages;
        let count = messages.len();

        let mut report = OutboxBatchReport {
            fetched: count,
            corrupted: fetch_result.corrupted_isolated,
            storage_errors: fetch_result.isolation_errors,
            ..Default::default()
        };

        if count == 0 {
            return Ok(report);
        }

        let worker_id = uuid::Uuid::new_v4().to_string();

        for message in messages {
            // Lock the message strictly
            let claim_result = self
                .outbox_repo
                .mark_processing(&message.id, &worker_id)
                .await;

            let claimed = match claim_result {
                Ok(c) => c,
                Err(_e) => {
                    tracing::error!(
                        error = %common::observability::redaction::DiagnosticError {
                            kind: "OutboxClaimFailed",
                            code: None,
                            retryable: true,
                        },
                        message_id = %message.id,
                        "OutboxWorker: Failed to claim message"
                    );
                    report.storage_errors += 1;
                    continue;
                }
            };

            if !claimed {
                // Legitimate claim conflict
                report.claim_conflicts += 1;
                continue;
            }

            report.claimed += 1;

            let result = self
                .handler
                .process_payload(&message.id, &message.payload)
                .await;

            match result {
                Ok(_) => match self.outbox_repo.mark_done(&message.id).await {
                    Ok(_) => {
                        report.completed += 1;
                    }
                    Err(_e) => {
                        tracing::error!(
                            error = %common::observability::redaction::DiagnosticError {
                                kind: "OutboxMarkDoneFailed",
                                code: None,
                                retryable: true,
                            },
                            message_id = %message.id,
                            "OutboxWorker: Failed to mark message as done"
                        );
                        report.storage_errors += 1;
                    }
                },
                Err(e) => {
                    let err_str = e.to_string();
                    match self.outbox_repo.mark_failed(&message.id, &err_str).await {
                        Ok(_) => {
                            // Can't trivially know if it became dead or retry_scheduled just from mark_failed Ok,
                            // but we record that an error was handled correctly by storage.
                            // We will increment retry_scheduled as an approximation, or both.
                            // If attempts >= 4 previously, it's dead, else retry_scheduled.
                            if message.attempts >= 4 {
                                report.dead += 1;
                            } else {
                                report.retry_scheduled += 1;
                            }
                        }
                        Err(_db_err) => {
                            tracing::error!(
                                error = %common::observability::redaction::DiagnosticError {
                                    kind: "OutboxMarkFailedError",
                                    code: None,
                                    retryable: true,
                                },
                                message_id = %message.id,
                                "OutboxWorker: Failed to mark message as failed"
                            );
                            report.storage_errors += 1;
                        }
                    }
                }
            }
        }

        Ok(report)
    }
}
