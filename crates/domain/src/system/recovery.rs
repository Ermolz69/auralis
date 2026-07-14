use crate::job::JobId;
use crate::project::ProjectId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryIssueType {
    OrphanActiveJob,
    MultipleActiveJobs,
    MissingActiveJob,
    JobProjectMismatch,
    MissingLegacyJob,
}

impl std::fmt::Display for RecoveryIssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OrphanActiveJob => {
                write!(f, "Orphan active job not attached to processing project")
            }
            Self::MultipleActiveJobs => {
                write!(f, "Multiple active jobs found for the same project")
            }
            Self::MissingActiveJob => write!(f, "Processing project is missing its active job"),
            Self::JobProjectMismatch => {
                write!(f, "Job's project ID does not match active_job_id owner")
            }
            Self::MissingLegacyJob => write!(
                f,
                "Legacy project missing active_job_id, unable to resolve unambiguously"
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RecoveryWarning {
    pub project_id: Option<ProjectId>,
    pub job_id: Option<JobId>,
    pub issue_type: RecoveryIssueType,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct RecoveryFatalIssue {
    pub project_id: Option<ProjectId>,
    pub job_id: Option<JobId>,
    pub issue_type: RecoveryIssueType,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct RecoveryReport {
    pub recovered_pairs: usize,
    pub reconciled_terminal_projects: usize,
    pub recovered_orphan_jobs: usize,
    pub warnings: Vec<RecoveryWarning>,
    pub fatal_issues: Vec<RecoveryFatalIssue>,
}

impl RecoveryReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_fatal_issues(&self) -> bool {
        !self.fatal_issues.is_empty()
    }

    pub fn add_warning(&mut self, warning: RecoveryWarning) {
        self.warnings.push(warning);
    }

    pub fn add_fatal_issue(&mut self, issue: RecoveryFatalIssue) {
        self.fatal_issues.push(issue);
    }
}
