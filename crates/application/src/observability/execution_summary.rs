use std::time::Instant;
use tracing::{Span, error, info, warn};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationSummary {
    JobExecution {
        project_id: String,
        job_id: String,
        action: &'static str,
        status: String,
    },
    Recovery {
        action: &'static str,
        status: String,
        actions_applied: u64,
        resolved_count: u64,
        unresolved_count: u64,
        failed_count: u64,
    },
    ProjectDeletion {
        project_id: String,
        action: &'static str,
        status: String,
        deleted_count: u64,
        failed_count: u64,
    },
    OutboxBatch {
        action: &'static str,
        status: String,
        failed_count: u64,
    },
    Maintenance {
        action: &'static str,
        status: String,
        deleted_count: u64,
        failed_count: u64,
    },
}

impl OperationSummary {
    pub fn update_status(&mut self, new_status: &str) {
        match self {
            Self::JobExecution { status, .. } => *status = new_status.to_string(),
            Self::Recovery { status, .. } => *status = new_status.to_string(),
            Self::ProjectDeletion { status, .. } => *status = new_status.to_string(),
            Self::OutboxBatch { status, .. } => *status = new_status.to_string(),
            Self::Maintenance { status, .. } => *status = new_status.to_string(),
        }
    }
}

pub struct ExecutionSummaryGuard {
    span: Span,
    start: Instant,
    pub summary: OperationSummary,
}

impl ExecutionSummaryGuard {
    pub fn new(span: Span, summary: OperationSummary) -> Self {
        Self {
            span,
            start: Instant::now(),
            summary,
        }
    }

    pub fn update_summary(&mut self, new_summary: OperationSummary) {
        self.summary = new_summary;
    }
}

impl Drop for ExecutionSummaryGuard {
    fn drop(&mut self) {
        if std::thread::panicking() {
            self.summary.update_status("panicked");
        }

        let elapsed_ms = self.start.elapsed().as_millis() as u64;
        let summary_clone = self.summary.clone();

        self.span.in_scope(|| match summary_clone {
            OperationSummary::JobExecution {
                project_id,
                job_id,
                action,
                status,
            } => {
                let st = status.as_str();
                if st == "completed" || st == "cancelled" || st == "deleted" {
                    info!(
                        project_id,
                        job_id, action, status, elapsed_ms, "Job execution summary"
                    );
                } else if st == "completed_with_errors" {
                    warn!(
                        project_id,
                        job_id, action, status, elapsed_ms, "Job execution summary"
                    );
                } else {
                    error!(
                        project_id,
                        job_id, action, status, elapsed_ms, "Job execution summary"
                    );
                }
            }
            OperationSummary::Recovery {
                action,
                status,
                actions_applied,
                resolved_count,
                unresolved_count,
                failed_count,
            } => {
                let st = status.as_str();
                if st == "completed" || st == "cancelled" || st == "deleted" {
                    info!(
                        action,
                        status,
                        actions_applied,
                        resolved_count,
                        unresolved_count,
                        failed_count,
                        elapsed_ms,
                        "Recovery summary"
                    );
                } else if st == "completed_with_errors" {
                    warn!(
                        action,
                        status,
                        actions_applied,
                        resolved_count,
                        unresolved_count,
                        failed_count,
                        elapsed_ms,
                        "Recovery summary"
                    );
                } else {
                    error!(
                        action,
                        status,
                        actions_applied,
                        resolved_count,
                        unresolved_count,
                        failed_count,
                        elapsed_ms,
                        "Recovery summary"
                    );
                }
            }
            OperationSummary::ProjectDeletion {
                project_id,
                action,
                status,
                deleted_count,
                failed_count,
            } => {
                let st = status.as_str();
                if st == "completed" || st == "cancelled" || st == "deleted" {
                    info!(
                        project_id,
                        action,
                        status,
                        deleted_count,
                        failed_count,
                        elapsed_ms,
                        "Project deletion summary"
                    );
                } else if st == "completed_with_errors" {
                    warn!(
                        project_id,
                        action,
                        status,
                        deleted_count,
                        failed_count,
                        elapsed_ms,
                        "Project deletion summary"
                    );
                } else {
                    error!(
                        project_id,
                        action,
                        status,
                        deleted_count,
                        failed_count,
                        elapsed_ms,
                        "Project deletion summary"
                    );
                }
            }
            OperationSummary::OutboxBatch {
                action,
                status,
                failed_count,
            } => {
                let st = status.as_str();
                if st == "completed" || st == "cancelled" || st == "deleted" {
                    info!(
                        action,
                        status, failed_count, elapsed_ms, "Outbox batch summary"
                    );
                } else if st == "completed_with_errors" {
                    warn!(
                        action,
                        status, failed_count, elapsed_ms, "Outbox batch summary"
                    );
                } else {
                    error!(
                        action,
                        status, failed_count, elapsed_ms, "Outbox batch summary"
                    );
                }
            }
            OperationSummary::Maintenance {
                action,
                status,
                deleted_count,
                failed_count,
            } => {
                let st = status.as_str();
                if st == "completed" || st == "cancelled" || st == "deleted" {
                    info!(
                        action,
                        status, deleted_count, failed_count, elapsed_ms, "Maintenance summary"
                    );
                } else if st == "completed_with_errors" {
                    warn!(
                        action,
                        status, deleted_count, failed_count, elapsed_ms, "Maintenance summary"
                    );
                } else {
                    error!(
                        action,
                        status, deleted_count, failed_count, elapsed_ms, "Maintenance summary"
                    );
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_summary_update_status() {
        let mut sum1 = OperationSummary::JobExecution {
            project_id: "p1".into(),
            job_id: "j1".into(),
            action: "exec",
            status: "aborted".into(),
        };
        sum1.update_status("completed");
        if let OperationSummary::JobExecution { status, .. } = sum1 {
            assert_eq!(status, "completed");
        } else {
            panic!("Wrong variant");
        }

        let mut sum2 = OperationSummary::Recovery {
            action: "rec",
            status: "aborted".into(),
            actions_applied: 0,
            resolved_count: 0,
            unresolved_count: 0,
            failed_count: 0,
        };
        sum2.update_status("failed");
        if let OperationSummary::Recovery { status, .. } = sum2 {
            assert_eq!(status, "failed");
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_execution_summary_guard_drop() {
        // Can only partially test drop without complex subscriber setup, but we verify it doesn't panic.
        let span = tracing::info_span!("test_span");
        let guard = ExecutionSummaryGuard::new(
            span,
            OperationSummary::JobExecution {
                project_id: "p1".into(),
                job_id: "j1".into(),
                action: "exec",
                status: "aborted".into(),
            },
        );
        drop(guard);
    }
}
