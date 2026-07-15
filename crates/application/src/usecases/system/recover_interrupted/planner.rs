use std::collections::{HashMap, HashSet};

use domain::job::{Job, JobId, JobStatus};
use domain::project::{Project, ProjectId};
use domain::system::recovery::{RecoveryIssueType, RecoveryViolation, RecoveryWarning};
use ports::recovery::RecoverySnapshot;

use super::report::RecoveryActionType;

pub enum RecoveryAction {
    FailInterruptedPair {
        project: Project,
        job: Job,
    },
    ReconcileTerminalPair {
        project: Project,
        job: Job,
    },
    FailLegacyPair {
        project: Project,
        job: Job,
    },
    FailProjectWithMissingLinkedJob {
        project: Project,
        missing_job_id: JobId,
    },
    FailLegacyProjectWithoutJob {
        project: Project,
    },
    FailOrphanJob {
        job: Job,
    },
}

impl RecoveryAction {
    pub fn action_type(&self) -> RecoveryActionType {
        match self {
            Self::FailInterruptedPair { .. } => RecoveryActionType::FailInterruptedPair,
            Self::ReconcileTerminalPair { .. } => RecoveryActionType::ReconcileTerminalPair,
            Self::FailLegacyPair { .. } => RecoveryActionType::FailLegacyPair,
            Self::FailProjectWithMissingLinkedJob { .. } => {
                RecoveryActionType::FailProjectWithMissingLinkedJob
            }
            Self::FailLegacyProjectWithoutJob { .. } => {
                RecoveryActionType::FailLegacyProjectWithoutJob
            }
            Self::FailOrphanJob { .. } => RecoveryActionType::FailOrphanJob,
        }
    }
}

pub struct RecoveryPlan {
    pub actions: Vec<RecoveryAction>,
    pub warnings: Vec<RecoveryWarning>,
    pub resolved_violations: Vec<RecoveryViolation>,
    pub unresolved_violations: Vec<RecoveryViolation>,
    pub blocked_project_ids: HashSet<ProjectId>,
    pub blocked_job_ids: HashSet<JobId>,
}

impl RecoveryPlan {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
            warnings: Vec::new(),
            resolved_violations: Vec::new(),
            unresolved_violations: Vec::new(),
            blocked_project_ids: HashSet::new(),
            blocked_job_ids: HashSet::new(),
        }
    }

    pub fn block_project(&mut self, id: ProjectId) {
        self.blocked_project_ids.insert(id);
    }

    pub fn block_job(&mut self, id: JobId) {
        self.blocked_job_ids.insert(id);
    }
}

impl Default for RecoveryPlan {
    fn default() -> Self {
        Self::new()
    }
}

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

        for job_ids in active_jobs_by_project.values() {
            if job_ids.len() > 1 {
                // Multiple active jobs for one project. Check if one of them is correctly linked.
                // Actually, the policy says:
                // "Нормальная active Job + доп. active Job того же Project -> нормальную пару восстановить, дополнительную Job сделать orphan, добавить warning."
                // So if we have a Processing project with active_job_id, we can spare it if there's only one.
                // Let's resolve this during project processing to see if one is explicitly linked.
                // Wait, if it's a legacy project with multiple active jobs, it's a fatal violation.
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
                                plan.actions.push(RecoveryAction::FailInterruptedPair {
                                    project,
                                    job: job.clone(),
                                });
                            } else {
                                plan.actions.push(RecoveryAction::ReconcileTerminalPair {
                                    project,
                                    job: job.clone(),
                                });
                            }
                        }
                        None => {
                            plan.actions
                                .push(RecoveryAction::FailProjectWithMissingLinkedJob {
                                    project,
                                    missing_job_id: active_job_id,
                                });
                        }
                    }
                }
                None => {
                    // Legacy fallback
                    let project_active_jobs: Vec<&Job> = active_jobs_map
                        .values()
                        .filter(|j| j.project_id() == project.id())
                        .collect();

                    if project_active_jobs.is_empty() {
                        plan.actions
                            .push(RecoveryAction::FailLegacyProjectWithoutJob { project });
                    } else if project_active_jobs.len() == 1 {
                        let job = project_active_jobs[0].clone();
                        processed_job_ids.insert(job.id().clone());
                        plan.actions
                            .push(RecoveryAction::FailLegacyPair { project, job });
                    } else {
                        plan.block_project(project.id().clone());
                        for j in project_active_jobs {
                            plan.block_job(j.id().clone());
                            processed_job_ids.insert(j.id().clone());
                        }
                        plan.unresolved_violations.push(RecoveryViolation {
                            project_id: Some(project.id().clone()),
                            job_id: None,
                            issue_type: RecoveryIssueType::AmbiguousLegacyJobs,
                            message: "Legacy project has multiple active jobs".into(),
                        });
                    }
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
                plan.actions.push(RecoveryAction::FailOrphanJob { job });
            }
        }

        plan
    }
}
