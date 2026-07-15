use crate::job::JobId;
use crate::project::ProjectId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryIssueType {
    OrphanActiveJob,
    MultipleActiveJobs,
    MissingActiveJob,
    JobProjectMismatch,
    AmbiguousLegacyJobs,
}

impl std::fmt::Display for RecoveryIssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OrphanActiveJob => {
                write!(f, "Orphan active job not attached to processing project")
            }
            Self::MultipleActiveJobs => write!(
                f,
                "Multiple active jobs found for the same project or duplicate links"
            ),
            Self::MissingActiveJob => write!(f, "Processing project is missing its active job"),
            Self::JobProjectMismatch => {
                write!(f, "Job's project ID does not match active_job_id owner")
            }
            Self::AmbiguousLegacyJobs => write!(
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
pub struct RecoveryViolation {
    pub project_id: Option<ProjectId>,
    pub job_id: Option<JobId>,
    pub issue_type: RecoveryIssueType,
    pub message: String,
}
