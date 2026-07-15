use domain::job::JobId;
use domain::project::ProjectId;
use domain::system::recovery::{RecoveryViolation, RecoveryWarning};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryActionType {
    FailInterruptedPair,
    ReconcileTerminalPair,
    FailLegacyPair,
    FailProjectWithMissingLinkedJob,
    FailLegacyProjectWithoutJob,
    FailOrphanJob,
}

#[derive(Debug, Clone)]
pub struct PersistenceFailure {
    pub action_kind: RecoveryActionType,
    pub project_id: Option<ProjectId>,
    pub job_id: Option<JobId>,
    pub error_type: String,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct RecoveryReport {
    pub actions_applied: usize,
    pub persistence_failures: Vec<PersistenceFailure>,
    pub resolved_violations: Vec<RecoveryViolation>,
    pub unresolved_violations: Vec<RecoveryViolation>,
    pub warnings: Vec<RecoveryWarning>,
}

impl RecoveryReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_blocking_failures(&self) -> bool {
        !self.persistence_failures.is_empty() || !self.unresolved_violations.is_empty()
    }

    pub fn add_persistence_failure(&mut self, failure: PersistenceFailure) {
        self.persistence_failures.push(failure);
    }

    pub fn add_resolved_violation(&mut self, violation: RecoveryViolation) {
        self.resolved_violations.push(violation);
    }

    pub fn add_unresolved_violation(&mut self, violation: RecoveryViolation) {
        self.unresolved_violations.push(violation);
    }

    pub fn add_warning(&mut self, warning: RecoveryWarning) {
        self.warnings.push(warning);
    }

    pub fn increment_actions(&mut self) {
        self.actions_applied += 1;
    }
}
