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

use super::maintenance::{
    MaintenanceCoordinator, OutboxMaintenanceConfig, OutboxMaintenanceReport,
};
use tokio::sync::watch;
use tokio::task::JoinSet;

pub struct OutboxWorker<O, S, I, U>
where
    O: OutboxRepository,
    S: ArtifactStore + Clone,
    I: ArtifactIndex + Clone,
    U: StorageUnitOfWork,
{
    outbox_repo: O,
    handler: PayloadHandler<S, I, U>,
    config: OutboxMaintenanceConfig,
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
        }
    }

    pub async fn process_pending_messages(
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
                Err(e) => {
                    tracing::error!(
                        message_id = %message.id,
                        "OutboxWorker: Failed to claim message: {}",
                        e
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
                    Err(e) => {
                        tracing::error!(
                            message_id = %message.id,
                            "OutboxWorker: Failed to mark message as done: {}",
                            e
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
                            tracing::error!(
                                message_id = %message.id,
                                "OutboxWorker: Failed to mark message as failed: {}",
                                db_err
                            );
                            report.storage_errors += 1;
                        }
                    }
                }
            }
        }

        Ok(report)
    }

    pub async fn run_loop(self: Arc<Self>, mut shutdown_rx: tokio::sync::mpsc::Receiver<()>)
    where
        O: Clone + Send + Sync + 'static,
        S: Clone + Send + Sync + 'static,
    {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let (cancel_tx, cancel_rx) = watch::channel(false);
        let mut maintenance_join_set = JoinSet::<OutboxMaintenanceReport>::new();

        if self.config.run_on_startup {
            let coordinator = MaintenanceCoordinator::new(
                self.outbox_repo.clone(),
                self.handler.artifact_store.clone(),
                self.handler.workspace_port.clone(),
                self.config.clone(),
            );
            let token = cancel_rx.clone();
            maintenance_join_set.spawn(async move { coordinator.run_maintenance(token).await });
        }

        let now = tokio::time::Instant::now();
        let mut maintenance_interval =
            tokio::time::interval_at(now + self.config.interval, self.config.interval);
        maintenance_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    match self.process_pending_messages(10).await {
                        Ok(report) => {
                            if report.fetched > 0 || report.storage_errors > 0 {
                                tracing::info!("OutboxWorker report: {}", report);
                            }
                        }
                        Err(e) => {
                            tracing::error!("OutboxWorker error: {}", e);
                        }
                    }
                }
                _ = maintenance_interval.tick() => {
                    if maintenance_join_set.is_empty() {
                        let coordinator = MaintenanceCoordinator::new(
                            self.outbox_repo.clone(),
                            self.handler.artifact_store.clone(),
                            self.handler.workspace_port.clone(),
                            self.config.clone(),
                        );
                        let token = cancel_rx.clone();
                        maintenance_join_set.spawn(async move {
                            coordinator.run_maintenance(token).await
                        });
                    } else {
                        tracing::warn!("OutboxWorker: skipping maintenance tick because previous maintenance is still running");
                    }
                }
                Some(result) = maintenance_join_set.join_next() => {
                    match result {
                        Ok(report) => {
                            tracing::info!(
                                ?report,
                                "OutboxWorker: maintenance completed"
                            );
                        }
                        Err(e) => {
                            if e.is_panic() {
                                tracing::error!("OutboxWorker: maintenance task panicked!");
                            } else if e.is_cancelled() {
                                tracing::warn!("OutboxWorker: maintenance task was cancelled");
                            } else {
                                tracing::error!("OutboxWorker: maintenance task failed: {}", e);
                            }
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("OutboxWorker shutting down...");
                    let _ = cancel_tx.send(true);

                    if !maintenance_join_set.is_empty() {
                        tracing::info!("OutboxWorker: waiting for maintenance task to cancel gracefully...");
                        let timeout = tokio::time::timeout(self.config.shutdown_timeout, async {
                            while let Some(res) = maintenance_join_set.join_next().await {
                                tracing::info!("OutboxWorker: maintenance shutdown report: {:?}", res);
                            }
                        });

                        if timeout.await.is_err() {
                            tracing::warn!("OutboxWorker: maintenance task did not shutdown gracefully, aborting");
                            maintenance_join_set.abort_all();
                            while maintenance_join_set.join_next().await.is_some() {}
                        }
                    }
                    break;
                }
            }
        }
    }
}
