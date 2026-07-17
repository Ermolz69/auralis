use domain::job::JobId;
use domain::project::ProjectId;

#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("Project not found: {0:?}")]
    ProjectNotFound(ProjectId),

    #[error("Job not found: {0:?}")]
    JobNotFound(JobId),

    #[error("Invalid operation: {message}")]
    InvalidOperation { message: String },

    #[error(
        "Pipeline start failed: scheduling failed with '{scheduling_error}', compensation failed with '{compensation_error}' (Recovery required)"
    )]
    PipelineStartFailedNeedsRecovery {
        scheduling_error: String,
        compensation_error: String,
    },

    #[error("Pipeline start failed: scheduling failed with '{scheduling_error}'")]
    PipelineStartFailed { scheduling_error: String },

    #[error("Unexpected error: {0}")]
    Unexpected(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Operation failed: {primary}, and cleanup also failed: {cleanup_report:?}")]
    OperationFailedWithCleanup {
        primary: Box<ApplicationError>,
        cleanup_report: CleanupReport,
    },

    #[error(transparent)]
    Domain(#[from] domain::error::DomainError),

    #[error(transparent)]
    Port(#[from] ports::error::PortError),
}

#[derive(Debug)]
pub enum CleanupTarget {
    Staging { key: String },
    Workspace { key: String },
}

#[derive(Debug)]
pub struct CleanupFailure {
    pub target: CleanupTarget,
    pub error: ports::error::PortError,
}

#[derive(Debug, Default)]
pub struct CleanupReport {
    pub failures: Vec<CleanupFailure>,
}

impl CleanupReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.failures.is_empty()
    }

    pub fn add_failure(&mut self, target: CleanupTarget, error: ports::error::PortError) {
        self.failures.push(CleanupFailure { target, error });
    }
}
