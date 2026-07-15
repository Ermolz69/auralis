use std::sync::Arc;
use std::time::Duration;

use ports::artifact_index::ArtifactIndex;
use ports::events::AppEventPublisher;
use ports::repository::OutboxRepository;
use ports::storage::ArtifactStore;
use ports::transaction::StorageUnitOfWork;
use ports::workspace::TempWorkspacePort;

use super::payload_handler::PayloadHandler;
use super::report::OutboxBatchReport;
use crate::error::ApplicationError;

pub struct OutboxWorker<O, S, I, U>
where
    O: OutboxRepository,
    S: ArtifactStore + Clone,
    I: ArtifactIndex + Clone,
    U: StorageUnitOfWork,
{
    outbox_repo: O,
    artifact_store: S,
    handler: PayloadHandler<S, I, U>,
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
            artifact_store,
            handler,
        }
    }

    pub async fn process_pending_messages(
        &self,
        limit: usize,
    ) -> Result<OutboxBatchReport, ApplicationError> {
        let messages = self.outbox_repo.fetch_pending(limit).await?;
        let count = messages.len();

        let mut report = OutboxBatchReport {
            fetched: count,
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
                Err(e) => {
                    eprintln!(
                        "OutboxWorker: Failed to claim message {}: {}",
                        message.id, e
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

            let result = self.handler.process_payload(&message.payload).await;

            match result {
                Ok(_) => match self.outbox_repo.mark_done(&message.id).await {
                    Ok(_) => {
                        report.completed += 1;
                    }
                    Err(e) => {
                        eprintln!(
                            "OutboxWorker: Failed to mark message {} as done: {}",
                            message.id, e
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
                        Err(db_err) => {
                            eprintln!(
                                "OutboxWorker: Failed to mark message {} as failed: {}",
                                message.id, db_err
                            );
                            report.storage_errors += 1;
                        }
                    }
                }
            }
        }

        Ok(report)
    }

    pub async fn run_loop(self: Arc<Self>, mut shutdown_rx: tokio::sync::mpsc::Receiver<()>) {
        let mut interval = tokio::time::interval(Duration::from_secs(5));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    match self.process_pending_messages(10).await {
                        Ok(report) => {
                            if report.fetched > 0 || report.storage_errors > 0 {
                                println!("OutboxWorker report: {}", report);
                            }
                        }
                        Err(e) => {
                            eprintln!("OutboxWorker error: {}", e);
                        }
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
