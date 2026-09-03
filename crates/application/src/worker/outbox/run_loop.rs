use std::sync::Arc;
use std::time::Duration;

use ports::artifact_index::ArtifactIndex;
use ports::repository::OutboxRepository;
use ports::storage::ArtifactStore;
use ports::transaction::StorageUnitOfWork;
use tokio::sync::watch;
use tokio::task::JoinSet;

use super::maintenance::{MaintenanceCoordinator, OutboxMaintenanceReport};
use super::worker::OutboxWorker;

impl<O, S, I, U> OutboxWorker<O, S, I, U>
where
    O: OutboxRepository,
    S: ArtifactStore + Clone,
    I: ArtifactIndex + Clone,
    U: StorageUnitOfWork,
{
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
            )
            .with_imports(self.imports.clone());
            let token = cancel_rx.clone();
            maintenance_join_set.spawn(async move { coordinator.run_maintenance(token).await });
        }

        let now = tokio::time::Instant::now();
        let mut maintenance_interval =
            tokio::time::interval_at(now + self.config.interval, self.config.interval);
        maintenance_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                biased;

                _ = shutdown_rx.recv() => {
                    tracing::info!("OutboxWorker shutting down...");
                    let _ = cancel_tx.send(true);

                    if !maintenance_join_set.is_empty() {
                        tracing::info!("OutboxWorker: waiting for maintenance task to cancel gracefully...");
                        while let Some(res) = maintenance_join_set.join_next().await {
                            tracing::info!("OutboxWorker: maintenance shutdown report: {:?}", res);
                        }
                    }
                    break;
                }
                _ = interval.tick() => {
                    match self.process_pending_messages(10).await {
                        Ok(report) => {
                            if report.fetched > 0 || report.storage_errors > 0 {
                                tracing::info!("OutboxWorker report: {}", report);
                            }
                        }
                        Err(_e) => {
                            tracing::error!(
                                error = %common::observability::redaction::DiagnosticError {
                                    kind: "OutboxProcessBatchFailed",
                                    code: None,
                                    retryable: true,
                                },
                                "OutboxWorker error"
                            );
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
                        ).with_imports(self.imports.clone());
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
                                tracing::error!(
                                    error = %common::observability::redaction::DiagnosticError {
                                        kind: "OutboxMaintenancePanicked",
                                        code: None,
                                        retryable: false,
                                    },
                                    "OutboxWorker: maintenance task panicked!"
                                );
                            } else if e.is_cancelled() {
                                tracing::warn!("OutboxWorker: maintenance task was cancelled");
                            } else {
                                tracing::error!(
                                    error = %common::observability::redaction::DiagnosticError {
                                        kind: "OutboxMaintenanceFailed",
                                        code: None,
                                        retryable: false,
                                    },
                                    "OutboxWorker: maintenance task failed"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
