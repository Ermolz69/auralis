use std::sync::Arc;

use domain::job::{JobError, TerminalOutcome};
use ports::recovery::{
    FailInterruptedPairCommand, FailLegacyPairFallbackCommand, FailLegacyProjectWithoutJobCommand,
    FailOrphanJobCommand, FailProjectWithMissingLinkedJobCommand, ReconcileTerminalPairCommand,
    RecoveryApplyResult, RecoveryStorage,
};

use crate::error::ApplicationError;

use super::planner::{Planner, RecoveryAction};
use super::report::{PersistenceFailure, RecoveryReport};

pub struct RecoverInterruptedStateUseCase {
    recovery_storage: Arc<dyn RecoveryStorage>,
}

impl RecoverInterruptedStateUseCase {
    pub fn new(recovery_storage: Arc<dyn RecoveryStorage>) -> Self {
        Self { recovery_storage }
    }

    pub fn execute(
        &self,
    ) -> impl std::future::Future<Output = Result<RecoveryReport, ApplicationError>> {
        let span = tracing::info_span!("recovery", action = "recovery");
        let mut guard = crate::observability::execution_summary::ExecutionSummaryGuard::new(
            span.clone(),
            crate::observability::execution_summary::OperationSummary::Recovery {
                action: "recovery",
                status: "aborted".to_string(),
                actions_applied: 0,
                resolved_count: 0,
                unresolved_count: 0,
                failed_count: 0,
            },
        );

        let storage = self.recovery_storage.clone();

        async move {
            let res = Self::execute_inner(storage).await;
            match &res {
                Ok(report) => {
                    let failed_count =
                        report.persistence_failures.len() + report.unresolved_violations.len();
                    let status = if failed_count > 0 {
                        "completed_with_errors"
                    } else {
                        "completed"
                    };
                    guard.update_summary(
                        crate::observability::execution_summary::OperationSummary::Recovery {
                            action: "recovery",
                            status: status.to_string(),
                            actions_applied: report.actions_applied as u64,
                            resolved_count: report.resolved_violations.len() as u64,
                            unresolved_count: report.unresolved_violations.len() as u64,
                            failed_count: failed_count as u64,
                        },
                    );
                }
                Err(_) => guard.summary.update_status("failed"),
            }
            res
        }
    }

    async fn execute_inner(
        recovery_storage: Arc<dyn RecoveryStorage>,
    ) -> Result<RecoveryReport, ApplicationError> {
        let snapshot = recovery_storage.load_snapshot().await.map_err(|e| {
            ApplicationError::Unexpected(format!("Failed to load recovery snapshot: {}", e))
        })?;

        let plan = Planner::build_plan(snapshot);
        let mut report = RecoveryReport::new();

        report.warnings = plan.warnings;
        report.unresolved_violations = plan.unresolved_violations;

        for planned in plan.actions {
            let action_kind = planned.action.action_type();
            let mut project_id_for_err = None;
            let mut job_id_for_err = None;

            let result = match planned.action {
                RecoveryAction::FailInterruptedPair {
                    mut project,
                    mut job,
                } => {
                    project_id_for_err = Some(project.id().clone());
                    job_id_for_err = Some(job.id().clone());

                    let expected_project_status = project.status().clone();
                    let expected_active_job_id = project.active_job_id().cloned().unwrap();
                    let expected_job_status = job.status().clone();
                    let expected_last_terminal_job_id = project.last_terminal_job_id().cloned();

                    if let Err(e) = job.mark_failed(JobError::new(
                        "APP_RESTART",
                        "Interrupted by application restart",
                        false,
                    )) {
                        Err(e.to_string())
                    } else if let Err(e) =
                        project.apply_terminal_transition(job.id(), TerminalOutcome::Failed)
                    {
                        Err(e.to_string())
                    } else {
                        recovery_storage
                            .commit_failed_interrupted_pair(FailInterruptedPairCommand {
                                project,
                                job,
                                expected_project_status,
                                expected_active_job_id,
                                expected_job_status,
                                expected_last_terminal_job_id,
                            })
                            .await
                            .map_err(|e| e.to_string())
                    }
                }
                RecoveryAction::ReconcileTerminalPair { mut project, job } => {
                    project_id_for_err = Some(project.id().clone());
                    job_id_for_err = Some(job.id().clone());

                    let expected_project_status = project.status().clone();
                    let expected_active_job_id = project.active_job_id().cloned().unwrap();
                    let expected_job_status = job.status().clone();
                    let expected_last_terminal_job_id = project.last_terminal_job_id().cloned();

                    let outcome = match *job.status() {
                        domain::job::JobStatus::Completed => TerminalOutcome::Completed,
                        domain::job::JobStatus::Failed => TerminalOutcome::Failed,
                        domain::job::JobStatus::Cancelled => TerminalOutcome::Cancelled,
                        _ => TerminalOutcome::Failed,
                    };

                    if let Err(e) = project.apply_terminal_transition(job.id(), outcome) {
                        Err(e.to_string())
                    } else {
                        recovery_storage
                            .commit_reconciled_terminal_pair(ReconcileTerminalPairCommand {
                                project,
                                job,
                                expected_project_status,
                                expected_active_job_id,
                                expected_job_status,
                                expected_last_terminal_job_id,
                            })
                            .await
                            .map_err(|e| e.to_string())
                    }
                }
                RecoveryAction::FailLegacyPair {
                    mut project,
                    mut job,
                } => {
                    project_id_for_err = Some(project.id().clone());
                    job_id_for_err = Some(job.id().clone());

                    let expected_project_status = project.status().clone();
                    let expected_job_status = job.status().clone();
                    let expected_last_terminal_job_id = project.last_terminal_job_id().cloned();

                    if let Err(e) = job.mark_failed(JobError::new(
                        "APP_RESTART",
                        "Interrupted by application restart (legacy fallback)",
                        false,
                    )) {
                        Err(e.to_string())
                    } else {
                        project.force_fail_legacy_recovery();
                        recovery_storage
                            .commit_legacy_pair_fallback(FailLegacyPairFallbackCommand {
                                project,
                                job,
                                expected_project_status,
                                expected_job_status,
                                expected_last_terminal_job_id,
                            })
                            .await
                            .map_err(|e| e.to_string())
                    }
                }
                RecoveryAction::FailProjectWithMissingLinkedJob {
                    mut project,
                    missing_job_id,
                } => {
                    project_id_for_err = Some(project.id().clone());
                    job_id_for_err = Some(missing_job_id.clone());

                    let expected_project_status = project.status().clone();
                    let expected_active_job_id = missing_job_id.clone();
                    let expected_last_terminal_job_id = project.last_terminal_job_id().cloned();

                    project.force_fail_legacy_recovery(); // Equivalent to force failing due to missing job
                    recovery_storage
                        .commit_failed_project_with_missing_linked_job(
                            FailProjectWithMissingLinkedJobCommand {
                                project,
                                expected_project_status,
                                expected_active_job_id,
                                expected_last_terminal_job_id,
                            },
                        )
                        .await
                        .map_err(|e| e.to_string())
                }
                RecoveryAction::FailLegacyProjectWithoutJob { mut project } => {
                    project_id_for_err = Some(project.id().clone());

                    let expected_project_status = project.status().clone();
                    let expected_last_terminal_job_id = project.last_terminal_job_id().cloned();

                    project.force_fail_legacy_recovery();
                    recovery_storage
                        .commit_failed_legacy_project_without_job(
                            FailLegacyProjectWithoutJobCommand {
                                project,
                                expected_project_status,
                                expected_last_terminal_job_id,
                            },
                        )
                        .await
                        .map_err(|e| e.to_string())
                }
                RecoveryAction::FailOrphanJob { mut job } => {
                    job_id_for_err = Some(job.id().clone());
                    let expected_job_status = job.status().clone();

                    if let Err(e) = job.mark_failed(JobError::new(
                        "APP_RESTART",
                        "Orphan active job interrupted by application restart",
                        false,
                    )) {
                        Err(e.to_string())
                    } else {
                        recovery_storage
                            .commit_failed_orphan_job(FailOrphanJobCommand {
                                job,
                                expected_job_status,
                            })
                            .await
                            .map_err(|e| e.to_string())
                    }
                }
            };

            match result {
                Ok(RecoveryApplyResult::Applied) => {
                    report.increment_actions();
                    if let Some(violation) = planned.resolved_violation {
                        report.resolved_violations.push(violation);
                    }
                }
                Ok(RecoveryApplyResult::AlreadyApplied) => {
                    if let Some(violation) = planned.resolved_violation {
                        report.resolved_violations.push(violation);
                    }
                }
                Err(e) => {
                    report.add_persistence_failure(PersistenceFailure {
                        action_kind,
                        project_id: project_id_for_err,
                        job_id: job_id_for_err,
                        error_type: "PersistenceFailure".to_string(),
                        message: e,
                    });
                }
            }
        }

        Ok(report)
    }
}
