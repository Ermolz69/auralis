use std::collections::HashMap;
use std::sync::Arc;

use domain::job::{JobError, JobStatus};
use domain::system::recovery::{
    RecoveryFatalIssue, RecoveryIssueType, RecoveryReport, RecoveryWarning,
};
use ports::recovery::RecoveryStorage;

pub struct RecoverInterruptedStateUseCase {
    recovery_storage: Arc<dyn RecoveryStorage>,
}

impl RecoverInterruptedStateUseCase {
    pub fn new(recovery_storage: Arc<dyn RecoveryStorage>) -> Self {
        Self { recovery_storage }
    }

    pub async fn execute(&self) -> Result<RecoveryReport, Box<dyn std::error::Error>> {
        let mut report = RecoveryReport::new();

        let mut snapshot = self.recovery_storage.load_snapshot().await?;
        let mut linked_jobs_map = HashMap::new();
        for job in snapshot.linked_jobs {
            linked_jobs_map.insert(job.id().clone(), job);
        }

        // 1. Process all Processing projects
        for mut project in snapshot.processing_projects {
            let active_job_id = project.active_job_id().cloned();

            match active_job_id {
                None => {
                    // Legacy fallback: check if there is EXACTLY ONE active job for this project
                    let project_active_jobs: Vec<_> = snapshot
                        .active_jobs
                        .iter()
                        .filter(|j| j.project_id() == project.id())
                        .collect();

                    if project_active_jobs.len() == 1 {
                        let mut job = project_active_jobs[0].clone();
                        let job_id = job.id().clone();

                        // We must remove this job from snapshot.active_jobs so it doesn't get orphan processing
                        snapshot.active_jobs.retain(|j| j.id() != &job_id);

                        let error = JobError::new(
                            "APP_RESTART",
                            "Interrupted by application restart (legacy fallback)",
                            false,
                        );
                        let _ = job.mark_failed(error);
                        let _ = project.apply_terminal_transition(
                            &job_id,
                            domain::job::TerminalOutcome::Failed,
                        );

                        if let Err(e) = self
                            .recovery_storage
                            .commit_interrupted_pair(project.clone(), job.clone())
                            .await
                        {
                            return Err(e.into());
                        }
                        report.recovered_pairs += 1;
                        report.add_warning(RecoveryWarning {
                            project_id: Some(project.id().clone()),
                            job_id: Some(job_id),
                            issue_type: RecoveryIssueType::MissingLegacyJob,
                            message: "Legacy project had no active_job_id but was recovered via exactly one active job match.".into(),
                        });
                    } else {
                        project.force_failed_due_to_recovery();
                        if let Err(e) = self
                            .recovery_storage
                            .commit_failed_project_no_job(project.clone())
                            .await
                        {
                            return Err(e.into());
                        }
                        report.add_warning(RecoveryWarning {
                            project_id: Some(project.id().clone()),
                            job_id: None,
                            issue_type: RecoveryIssueType::MissingLegacyJob,
                            message: format!("Legacy project had no active_job_id and found {} active jobs. Set to Failed.", project_active_jobs.len()),
                        });
                    }
                }
                Some(job_id) => {
                    match linked_jobs_map.remove(&job_id) {
                        Some(mut job) => {
                            if job.project_id() != project.id() {
                                report.add_fatal_issue(RecoveryFatalIssue {
                                    project_id: Some(project.id().clone()),
                                    job_id: Some(job.id().clone()),
                                    issue_type: RecoveryIssueType::JobProjectMismatch,
                                    message: "Job's project ID does not match active_job_id owner"
                                        .into(),
                                });
                                continue;
                            }

                            if matches!(*job.status(), JobStatus::Pending | JobStatus::Running) {
                                let error = JobError::new(
                                    "APP_RESTART",
                                    "Interrupted by application restart",
                                    false,
                                );
                                let _ = job.mark_failed(error);
                                let _ = project.apply_terminal_transition(
                                    &job_id,
                                    domain::job::TerminalOutcome::Failed,
                                );

                                if let Err(e) = self
                                    .recovery_storage
                                    .commit_interrupted_pair(project.clone(), job.clone())
                                    .await
                                {
                                    return Err(e.into());
                                }
                                report.recovered_pairs += 1;
                            } else {
                                // Job is already terminal (Completed/Failed/Cancelled). Sync Project.
                                let outcome = match *job.status() {
                                    JobStatus::Completed => domain::job::TerminalOutcome::Completed,
                                    JobStatus::Failed => domain::job::TerminalOutcome::Failed,
                                    JobStatus::Cancelled => domain::job::TerminalOutcome::Cancelled,
                                    _ => unreachable!(),
                                };
                                let _ = project.apply_terminal_transition(&job_id, outcome);
                                if let Err(e) = self
                                    .recovery_storage
                                    .commit_reconciled_project(project.clone())
                                    .await
                                {
                                    return Err(e.into());
                                }
                                report.reconciled_terminal_projects += 1;
                            }
                        }
                        None => {
                            // Missing linked job
                            project.force_failed_due_to_recovery();
                            if let Err(e) = self
                                .recovery_storage
                                .commit_failed_project_no_job(project.clone())
                                .await
                            {
                                return Err(e.into());
                            }
                            report.add_warning(RecoveryWarning {
                                project_id: Some(project.id().clone()),
                                job_id: Some(job_id),
                                issue_type: RecoveryIssueType::MissingActiveJob,
                                message: "Processing project's active_job_id pointed to a non-existent job. Set to Failed.".into(),
                            });
                        }
                    }
                }
            }
        }

        // 2. Process any remaining linked_jobs that weren't used.
        // Wait, if they were in `linked_jobs_map`, it means their ID was in `active_job_id` of SOME Processing project.
        // If we didn't remove it, it means the project wasn't Processing, but the query `load_snapshot` fetched it because we collected all `active_job_id`s.
        // Actually, in `load_snapshot`, we collected `active_job_id`s FROM Processing projects ONLY.
        // So `linked_jobs_map` should be exactly empty if all active_job_ids found their corresponding job.
        // If it's not empty, it means there are multiple Processing projects with the SAME active_job_id (which is a severe violation).
        for (job_id, _job) in linked_jobs_map {
            report.add_fatal_issue(RecoveryFatalIssue {
                project_id: None, // Multiple projects might claim this
                job_id: Some(job_id.clone()),
                issue_type: RecoveryIssueType::MultipleActiveJobs,
                message: "Multiple Processing projects point to the same active_job_id, or job was orphaned mysteriously.".into(),
            });
        }

        // 3. Process orphans (Pending/Running jobs not linked to a Processing project)
        for mut orphan_job in snapshot.active_jobs {
            let error = JobError::new(
                "APP_RESTART",
                "Orphan active job interrupted by application restart",
                false,
            );
            let _ = orphan_job.mark_failed(error);
            if let Err(e) = self
                .recovery_storage
                .commit_orphan_job(orphan_job.clone())
                .await
            {
                return Err(e.into());
            }
            report.recovered_orphan_jobs += 1;
            report.add_warning(RecoveryWarning {
                project_id: Some(orphan_job.project_id().clone()),
                job_id: Some(orphan_job.id().clone()),
                issue_type: RecoveryIssueType::OrphanActiveJob,
                message: "Orphan active job found and marked as Failed.".into(),
            });
        }

        Ok(report)
    }
}
