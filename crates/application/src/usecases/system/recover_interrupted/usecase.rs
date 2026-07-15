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

    pub async fn execute(&self) -> Result<RecoveryReport, ApplicationError> {
        let snapshot = self.recovery_storage.load_snapshot().await.map_err(|e| {
            ApplicationError::Unexpected(format!("Failed to load recovery snapshot: {}", e))
        })?;

        let plan = Planner::build_plan(snapshot);
        let mut report = RecoveryReport::new();

        report.warnings = plan.warnings;
        report.resolved_violations = plan.resolved_violations;
        report.unresolved_violations = plan.unresolved_violations;

        for action in plan.actions {
            let action_kind = action.action_type();
            let mut project_id_for_err = None;
            let mut job_id_for_err = None;

            let result = match action {
                RecoveryAction::FailInterruptedPair {
                    mut project,
                    mut job,
                } => {
                    project_id_for_err = Some(project.id().clone());
                    job_id_for_err = Some(job.id().clone());

                    let expected_project_status = project.status().clone();
                    let expected_active_job_id = project.active_job_id().cloned().unwrap();
                    let expected_job_status = job.status().clone();

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
                        self.recovery_storage
                            .commit_failed_interrupted_pair(FailInterruptedPairCommand {
                                project,
                                job,
                                expected_project_status,
                                expected_active_job_id,
                                expected_job_status,
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

                    let outcome = match *job.status() {
                        domain::job::JobStatus::Completed => TerminalOutcome::Completed,
                        domain::job::JobStatus::Failed => TerminalOutcome::Failed,
                        domain::job::JobStatus::Cancelled => TerminalOutcome::Cancelled,
                        _ => TerminalOutcome::Failed,
                    };

                    if let Err(e) = project.apply_terminal_transition(job.id(), outcome) {
                        Err(e.to_string())
                    } else {
                        self.recovery_storage
                            .commit_reconciled_terminal_pair(ReconcileTerminalPairCommand {
                                project,
                                job,
                                expected_project_status,
                                expected_active_job_id,
                                expected_job_status,
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

                    if let Err(e) = job.mark_failed(JobError::new(
                        "APP_RESTART",
                        "Interrupted by application restart (legacy fallback)",
                        false,
                    )) {
                        Err(e.to_string())
                    } else {
                        project.force_fail_legacy_recovery();
                        self.recovery_storage
                            .commit_legacy_pair_fallback(FailLegacyPairFallbackCommand {
                                project,
                                job,
                                expected_project_status,
                                expected_job_status,
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

                    project.force_fail_legacy_recovery(); // Equivalent to force failing due to missing job
                    self.recovery_storage
                        .commit_failed_project_with_missing_linked_job(
                            FailProjectWithMissingLinkedJobCommand {
                                project,
                                expected_project_status,
                                expected_active_job_id,
                            },
                        )
                        .await
                        .map_err(|e| e.to_string())
                }
                RecoveryAction::FailLegacyProjectWithoutJob { mut project } => {
                    project_id_for_err = Some(project.id().clone());

                    let expected_project_status = project.status().clone();

                    project.force_fail_legacy_recovery();
                    self.recovery_storage
                        .commit_failed_legacy_project_without_job(
                            FailLegacyProjectWithoutJobCommand {
                                project,
                                expected_project_status,
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
                        self.recovery_storage
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
                }
                Ok(RecoveryApplyResult::AlreadyApplied) => {
                    // Do nothing, already recovered
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
