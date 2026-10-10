use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use ports::artifact_finalization::ArtifactFinalizationLookup;
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
    I: ArtifactFinalizationLookup + Clone + 'static,
    U: StorageUnitOfWork + 'static,
{
    pub async fn run_loop(self: Arc<Self>, mut shutdown_rx: tokio::sync::mpsc::Receiver<()>)
    where
        O: Clone + Send + Sync + 'static,
        S: Clone + Send + Sync + 'static,
    {
        if let Err(error) = self.config.validate() {
            tracing::error!(error = %error, "OutboxWorker configuration is invalid");
            return;
        }
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let (cancel_tx, cancel_rx) = watch::channel(false);
        let mut maintenance_join_set = JoinSet::<OutboxMaintenanceReport>::new();
        let mut batch_join_set = JoinSet::new();

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
                    self.stopping.store(true, Ordering::Release);
                    let _ = cancel_tx.send(true);
                    let deadline = tokio::time::Instant::now() + self.config.shutdown_timeout;
                    if tokio::time::timeout_at(deadline, async {
                        while batch_join_set.join_next().await.is_some() {}
                        while maintenance_join_set.join_next().await.is_some() {}
                    }).await.is_err() {
                        batch_join_set.abort_all();
                        maintenance_join_set.abort_all();
                        while batch_join_set.join_next().await.is_some() {}
                        while maintenance_join_set.join_next().await.is_some() {}
                    }
                    break;
                }
                _ = interval.tick() => {
                    if batch_join_set.is_empty() {
                        let worker = self.clone();
                        batch_join_set.spawn(async move {
                            worker.process_pending_messages(worker.config.fetch_batch).await
                        });
                    }
                }
                Some(result) = batch_join_set.join_next() => match result {
                    Ok(Ok(report)) => {
                        if report.fetched > 0 || report.storage_errors > 0 {
                            tracing::info!("OutboxWorker report: {}", report);
                        }
                    }
                    Ok(Err(_)) | Err(_) => tracing::error!("OutboxWorker batch failed; claims remain recoverable"),
                },
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
