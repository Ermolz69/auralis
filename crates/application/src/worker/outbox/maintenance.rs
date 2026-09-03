pub use super::maintenance_config::OutboxMaintenanceConfig;
use ports::{repository::OutboxRepository, storage::ArtifactStore, workspace::TempWorkspacePort};
use std::sync::Arc;
use tokio::sync::watch;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MaintenanceStepOutcome {
    #[default]
    NotStarted,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct OutboxMaintenanceReport {
    pub staging_cleanup: MaintenanceStepOutcome,
    pub workspace_cleanup: MaintenanceStepOutcome,
    pub pruning: MaintenanceStepOutcome,
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
    imports: Option<Arc<dyn ports::youtube_import::YoutubeImportJournal>>,
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
            imports: None,
        }
    }

    pub fn with_imports(
        mut self,
        imports: Option<Arc<dyn ports::youtube_import::YoutubeImportJournal>>,
    ) -> Self {
        self.imports = imports;
        self
    }

    pub fn run_maintenance<'a>(
        &'a self,
        cancel_token: watch::Receiver<bool>,
    ) -> impl std::future::Future<Output = OutboxMaintenanceReport> + 'a {
        self.run_maintenance_at(domain::chrono::Utc::now(), cancel_token)
    }

    pub(crate) fn run_maintenance_at<'a>(
        &'a self,
        now: domain::chrono::DateTime<domain::chrono::Utc>,
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
            let report = self.run_maintenance_inner_at(now, cancel_token).await;

            let mut failed_count = 0;
            if report.staging_cleanup == MaintenanceStepOutcome::Failed {
                failed_count += 1;
            }
            if report.workspace_cleanup == MaintenanceStepOutcome::Failed {
                failed_count += 1;
            }
            if report.pruning == MaintenanceStepOutcome::Failed {
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

    async fn run_maintenance_inner_at(
        &self,
        now: domain::chrono::DateTime<domain::chrono::Utc>,
        cancel_token: watch::Receiver<bool>,
    ) -> OutboxMaintenanceReport {
        let mut report = OutboxMaintenanceReport::default();
        let done_before = now - self.config.done_retention;
        let dead_before = now - self.config.dead_retention;
        let sessions = match &self.imports {
            Some(imports) => match imports.list().await {
                Ok(sessions) => sessions,
                Err(_) => {
                    report.staging_cleanup = MaintenanceStepOutcome::Failed;
                    report.workspace_cleanup = MaintenanceStepOutcome::Failed;
                    return report;
                }
            },
            None => Vec::new(),
        };
        let staging = match &self.imports {
            Some(imports) => match imports.protected_staging_keys().await {
                Ok(keys) => keys,
                Err(_) => {
                    report.staging_cleanup = MaintenanceStepOutcome::Failed;
                    report.workspace_cleanup = MaintenanceStepOutcome::Failed;
                    return report;
                }
            },
            None => Vec::new(),
        };
        let workspaces: Vec<_> = sessions
            .into_iter()
            .map(|session| session.workspace_key)
            .collect();

        if *cancel_token.borrow() {
            report.cancelled = true;
            return report;
        }

        // 1. Staging janitor
        report.staging_cleanup = MaintenanceStepOutcome::NotStarted;
        if *cancel_token.borrow() {
            report.cancelled = true;
            return report;
        }
        match self
            .artifact_store
            .cleanup_stale_staging_excluding(self.config.staging_max_age, &staging)
            .await
        {
            Ok(_) => {
                report.staging_cleanup = MaintenanceStepOutcome::Succeeded;
            }
            Err(_) => {
                report.staging_cleanup = MaintenanceStepOutcome::Failed;
            }
        }

        if *cancel_token.borrow() {
            report.cancelled = true;
            return report;
        }

        // 2. Workspace janitor
        report.workspace_cleanup = MaintenanceStepOutcome::NotStarted;
        match self
            .workspace_provider
            .cleanup_stale_allocations_excluding(self.config.workspace_max_age, &workspaces)
            .await
        {
            Ok(_) => {
                report.workspace_cleanup = MaintenanceStepOutcome::Succeeded;
            }
            Err(_) => {
                report.workspace_cleanup = MaintenanceStepOutcome::Failed;
            }
        }

        if *cancel_token.borrow() {
            report.cancelled = true;
            return report;
        }

        // 3. Pruning loop
        report.pruning = MaintenanceStepOutcome::NotStarted;
        for _ in 0..self.config.max_batches {
            if *cancel_token.borrow() {
                report.cancelled = true;
                report.pruning = MaintenanceStepOutcome::Cancelled;
                break;
            }

            tokio::task::yield_now().await;

            match self
                .outbox_repo
                .prune_terminal_rows(done_before, dead_before, self.config.per_status_batch_limit)
                .await
            {
                Ok(prune_report) => {
                    report.pruning = MaintenanceStepOutcome::Succeeded;
                    report.done_deleted += prune_report.done_deleted;
                    report.dead_deleted += prune_report.dead_deleted;
                    report.batches_run += 1;

                    if prune_report.done_deleted < self.config.per_status_batch_limit as usize
                        && prune_report.dead_deleted < self.config.per_status_batch_limit as usize
                    {
                        break;
                    }
                }
                Err(_) => {
                    report.pruning = MaintenanceStepOutcome::Failed;
                    break;
                }
            }
        }

        report
    }
}
