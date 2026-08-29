use std::collections::{HashMap, HashSet};

use domain::job::{Job, JobId, JobStatus};
use domain::project::ProjectId;
use domain::system::recovery::{RecoveryIssueType, RecoveryViolation, RecoveryWarning};
use ports::recovery::RecoverySnapshot;

use super::actions::{PlannedAction, RecoveryAction, RecoveryPlan};

pub struct Planner;

impl Planner {
    pub fn build_plan(snapshot: RecoverySnapshot) -> RecoveryPlan {
        let mut plan = RecoveryPlan::new();

        let mut linked_jobs_map: HashMap<JobId, Job> = HashMap::new();
        for job in snapshot.linked_jobs {
            linked_jobs_map.insert(job.id().clone(), job);
        }

        let mut active_jobs_map: HashMap<JobId, Job> = HashMap::new();
        for job in snapshot.active_jobs {
            active_jobs_map.insert(job.id().clone(), job);
        }

        // 1. Find conflicting active jobs for the same project
        let mut active_jobs_by_project: HashMap<ProjectId, Vec<JobId>> = HashMap::new();
        for job in active_jobs_map.values().chain(linked_jobs_map.values()) {
            if matches!(*job.status(), JobStatus::Pending | JobStatus::Running) {
                active_jobs_by_project
                    .entry(job.project_id().clone())
                    .or_default()
                    .push(job.id().clone());
            }
        }

        // 2. Identify duplicate active_job_id usage across projects
        let mut active_job_id_users: HashMap<JobId, Vec<ProjectId>> = HashMap::new();
        for project in &snapshot.processing_projects {
            if let Some(active_id) = project.active_job_id() {
                active_job_id_users
                    .entry(active_id.clone())
                    .or_default()
                    .push(project.id().clone());
            }
        }

        for (job_id, project_ids) in active_job_id_users {
            if project_ids.len() > 1 {
                for pid in project_ids {
                    plan.block_project(pid.clone());
                }
                plan.block_job(job_id.clone());
                plan.unresolved_violations.push(RecoveryViolation {
                    project_id: None,
                    job_id: Some(job_id.clone()),
                    issue_type: RecoveryIssueType::MultipleActiveJobs,
                    message: "Multiple projects point to the same active_job_id".into(),
                });
            }
        }

        // 3. Process Projects
        let mut processed_job_ids: HashSet<JobId> = HashSet::new();

        for project in snapshot.processing_projects {
            if plan.blocked_project_ids.contains(project.id()) {
                continue;
            }

            match project.active_job_id().cloned() {
                Some(active_job_id) => {
                    processed_job_ids.insert(active_job_id.clone());

                    if plan.blocked_job_ids.contains(&active_job_id) {
                        continue;
                    }

                    match linked_jobs_map.get(&active_job_id) {
                        Some(job) => {
                            if job.project_id() != project.id() {
                                plan.block_project(project.id().clone());
                                plan.block_job(job.id().clone());
                                plan.unresolved_violations.push(RecoveryViolation {
                                    project_id: Some(project.id().clone()),
                                    job_id: Some(job.id().clone()),
                                    issue_type: RecoveryIssueType::JobProjectMismatch,
                                    message: "Job belongs to another project".into(),
                                });
                                continue;
                            }

                            // Normal pair
                            let additional_active = active_jobs_by_project
                                .get(project.id())
                                .map(|ids| ids.iter().filter(|id| **id != active_job_id).count())
                                .unwrap_or(0);

                            if additional_active > 0 {
                                plan.warnings.push(RecoveryWarning {
                                    project_id: Some(project.id().clone()),
                                    job_id: Some(active_job_id.clone()),
                                    issue_type: RecoveryIssueType::MultipleActiveJobs,
                                    message: "Project has additional orphan active jobs".into(),
                                });
                            }

                            if matches!(*job.status(), JobStatus::Pending | JobStatus::Running) {
                                plan.actions.push(PlannedAction {
                                    action: RecoveryAction::FailInterruptedPair {
                                        project,
                                        job: job.clone(),
                                        active_job_id: active_job_id.clone(),
                                    },
                                    resolved_violation: None,
                                });
                            } else {
                                plan.actions.push(PlannedAction {
                                    action: RecoveryAction::ReconcileTerminalPair {
                                        project,
                                        job: job.clone(),
                                        active_job_id: active_job_id.clone(),
                                    },
                                    resolved_violation: None,
                                });
                            }
                        }
                        None => {
                            plan.actions.push(PlannedAction {
                                action: RecoveryAction::FailProjectWithMissingLinkedJob {
                                    project: project.clone(),
                                    missing_job_id: active_job_id.clone(),
                                },
                                resolved_violation: Some(RecoveryViolation {
                                    project_id: Some(project.id().clone()),
                                    job_id: Some(active_job_id),
                                    issue_type: RecoveryIssueType::MissingActiveJob,
                                    message: "Project points to a missing job".into(),
                                }),
                            });
                        }
                    }
                }
                None => {
                    plan.actions.push(PlannedAction {
                        action: RecoveryAction::FailProjectWithoutActiveJob {
                            project: project.clone(),
                        },
                        resolved_violation: Some(RecoveryViolation {
                            project_id: Some(project.id().clone()),
                            job_id: None,
                            issue_type: RecoveryIssueType::MissingActiveJob,
                            message: "Processing project has no active job link".into(),
                        }),
                    });
                }
            }
        }

        // 4. Process remaining jobs (Orphans)
        for job in linked_jobs_map
            .into_values()
            .chain(active_jobs_map.into_values())
        {
            if plan.blocked_job_ids.contains(job.id()) || processed_job_ids.contains(job.id()) {
                continue;
            }

            if matches!(*job.status(), JobStatus::Pending | JobStatus::Running) {
                plan.actions.push(PlannedAction {
                    action: RecoveryAction::FailOrphanJob { job: job.clone() },
                    resolved_violation: Some(RecoveryViolation {
                        project_id: Some(job.project_id().clone()),
                        job_id: Some(job.id().clone()),
                        issue_type: RecoveryIssueType::OrphanActiveJob,
                        message: "Active job has no owning project".into(),
                    }),
                });
            }
        }

        plan
    }
}
