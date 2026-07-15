use ports::repository::OutboxRepository;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::time::{Duration, interval};

pub struct StorageMaintenanceWorker {
    outbox_repo: Arc<dyn OutboxRepository>,
}

impl StorageMaintenanceWorker {
    pub fn new(outbox_repo: Arc<dyn OutboxRepository>) -> Self {
        Self { outbox_repo }
    }

    pub async fn run(self, mut shutdown_rx: oneshot::Receiver<()>) {
        let mut interval = interval(Duration::from_secs(60 * 60 * 24)); // Run daily
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        tracing::info!("StorageMaintenanceWorker started");

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.run_prune_task().await;
                }
                _ = &mut shutdown_rx => {
                    tracing::info!("StorageMaintenanceWorker received shutdown signal, stopping");
                    break;
                }
            }
        }
    }

    async fn run_prune_task(&self) {
        let done_retention_days = 7;
        let dead_retention_days = 30;

        tracing::info!(
            "StorageMaintenanceWorker: starting prune of outbox messages (done > {} days, dead > {} days)",
            done_retention_days,
            dead_retention_days
        );

        match self
            .outbox_repo
            .prune_terminal_rows(done_retention_days, dead_retention_days)
            .await
        {
            Ok(report) => {
                tracing::info!(
                    done_deleted = report.done_deleted,
                    dead_deleted = report.dead_deleted,
                    "StorageMaintenanceWorker: outbox prune completed successfully"
                );
            }
            Err(e) => {
                tracing::error!(
                    error = ?e,
                    "StorageMaintenanceWorker: failed to prune outbox messages"
                );
            }
        }
    }
}
