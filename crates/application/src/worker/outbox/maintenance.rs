use ports::{repository::OutboxRepository, storage::ArtifactStore, workspace::TempWorkspacePort};
use std::{sync::Arc, time::Duration};
use tokio::sync::watch;

#[derive(Debug, Clone)]
pub struct OutboxMaintenanceConfig {
    pub interval: Duration,
    pub staging_max_age: Duration,
    pub workspace_max_age: Duration,
    pub done_retention: domain::chrono::TimeDelta,
    pub dead_retention: domain::chrono::TimeDelta,
    pub batch_limit: u32,
    pub max_batches: u32,
    pub run_on_startup: bool,
    pub shutdown_timeout: Duration,
}

impl OutboxMaintenanceConfig {
    pub fn default_config() -> Self {
        Self {
            interval: Duration::from_secs(3600),
            staging_max_age: Duration::from_secs(86400),
            workspace_max_age: Duration::from_secs(86400),
            done_retention: domain::chrono::TimeDelta::try_days(7).unwrap(),
            dead_retention: domain::chrono::TimeDelta::try_days(30).unwrap(),
            batch_limit: 500,
            max_batches: 10,
            run_on_startup: true,
            shutdown_timeout: Duration::from_secs(30),
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.interval.is_zero() {
            return Err("interval must be non-zero");
        }
        if self.batch_limit == 0 {
            return Err("batch_limit must be non-zero");
        }
        if self.max_batches == 0 {
            return Err("max_batches must be non-zero");
        }
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct OutboxMaintenanceReport {
    pub staging_cleanup_error: Option<String>,
    pub workspace_cleanup_error: Option<String>,
    pub prune_error: Option<String>,
    pub done_deleted: usize,
    pub dead_deleted: usize,
    pub batches_run: u32,
    pub cancelled: bool,
}

pub struct MaintenanceCoordinator<O, S> {
    outbox_repo: O,
    artifact_store: S,
    workspace_provider: Arc<dyn TempWorkspacePort>,
    config: OutboxMaintenanceConfig,
}

impl<O, S> MaintenanceCoordinator<O, S>
where
    O: OutboxRepository,
    S: ArtifactStore,
{
    pub fn new(
        outbox_repo: O,
        artifact_store: S,
        workspace_provider: Arc<dyn TempWorkspacePort>,
        config: OutboxMaintenanceConfig,
    ) -> Self {
        Self {
            outbox_repo,
            artifact_store,
            workspace_provider,
            config,
        }
    }

    pub fn run_maintenance<'a>(
        &'a self,
        cancel_token: watch::Receiver<bool>,
    ) -> impl std::future::Future<Output = OutboxMaintenanceReport> + 'a {
        let span = tracing::info_span!("maintenance_run", action = "maintenance_run");
        let mut guard = crate::observability::execution_summary::ExecutionSummaryGuard::new(
            span.clone(),
            crate::observability::execution_summary::OperationSummary::Maintenance {
                action: "maintenance_run",
                status: "aborted".to_string(),
                deleted_count: 0,
                failed_count: 0,
            },
        );

        async move {
            let report = self.run_maintenance_inner(cancel_token).await;

            let mut failed_count = 0;
            if report.staging_cleanup_error.is_some() {
                failed_count += 1;
            }
            if report.workspace_cleanup_error.is_some() {
                failed_count += 1;
            }
            if report.prune_error.is_some() {
                failed_count += 1;
            }

            let status = if report.cancelled {
                "cancelled"
            } else if failed_count > 0 {
                "completed_with_errors"
            } else {
                "completed"
            };

            guard.update_summary(
                crate::observability::execution_summary::OperationSummary::Maintenance {
                    action: "maintenance_run",
                    status: status.to_string(),
                    deleted_count: (report.done_deleted + report.dead_deleted) as u64,
                    failed_count,
                },
            );

            report
        }
    }

    async fn run_maintenance_inner(
        &self,
        cancel_token: watch::Receiver<bool>,
    ) -> OutboxMaintenanceReport {
        let mut report = OutboxMaintenanceReport::default();
        let now = domain::chrono::Utc::now();
        let done_before = now - self.config.done_retention;
        let dead_before = now - self.config.dead_retention;

        if *cancel_token.borrow() {
            report.cancelled = true;
            return report;
        }

        // 1. Staging janitor
        if let Err(e) = self
            .artifact_store
            .cleanup_stale_staging(self.config.staging_max_age)
            .await
        {
            report.staging_cleanup_error = Some(e.to_string());
        }

        if *cancel_token.borrow() {
            report.cancelled = true;
            return report;
        }

        // 2. Workspace janitor
        if let Err(e) = self
            .workspace_provider
            .cleanup_stale_allocations(self.config.workspace_max_age)
            .await
        {
            report.workspace_cleanup_error = Some(e.to_string());
        }

        // 3. Pruning loop
        for _ in 0..self.config.max_batches {
            if *cancel_token.borrow() {
                report.cancelled = true;
                break;
            }

            tokio::task::yield_now().await;

            match self
                .outbox_repo
                .prune_terminal_rows(done_before, dead_before, self.config.batch_limit)
                .await
            {
                Ok(prune_report) => {
                    report.done_deleted += prune_report.done_deleted;
                    report.dead_deleted += prune_report.dead_deleted;
                    report.batches_run += 1;

                    if prune_report.done_deleted < self.config.batch_limit as usize
                        && prune_report.dead_deleted < self.config.batch_limit as usize
                    {
                        break;
                    }
                }
                Err(e) => {
                    report.prune_error = Some(e.to_string());
                    break;
                }
            }
        }

        report
    }
}
