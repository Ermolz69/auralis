use std::collections::HashSet;

use domain::job::{Job, JobId};
use domain::project::{Project, ProjectId};
use domain::system::recovery::{RecoveryViolation, RecoveryWarning};

use super::report::RecoveryActionType;

pub struct PlannedAction {
    pub action: RecoveryAction,
    pub resolved_violation: Option<RecoveryViolation>,
}

pub enum RecoveryAction {
    FailInterruptedPair {
        project: Project,
        job: Job,
        active_job_id: JobId,
    },
    ReconcileTerminalPair {
        project: Project,
        job: Job,
        active_job_id: JobId,
    },
    FailProjectWithMissingLinkedJob {
        project: Project,
        missing_job_id: JobId,
    },
    FailProjectWithoutActiveJob {
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
            Self::FailProjectWithMissingLinkedJob { .. } => {
                RecoveryActionType::FailProjectWithMissingLinkedJob
            }
            Self::FailProjectWithoutActiveJob { .. } => {
                RecoveryActionType::FailProjectWithoutActiveJob
            }
            Self::FailOrphanJob { .. } => RecoveryActionType::FailOrphanJob,
        }
    }
}

pub struct RecoveryPlan {
    pub actions: Vec<PlannedAction>,
    pub warnings: Vec<RecoveryWarning>,
    pub unresolved_violations: Vec<RecoveryViolation>,
    pub blocked_project_ids: HashSet<ProjectId>,
    pub blocked_job_ids: HashSet<JobId>,
}

impl RecoveryPlan {
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
            warnings: Vec::new(),
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
