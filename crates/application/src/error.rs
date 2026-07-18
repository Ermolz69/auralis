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

    #[error("Operation failed: {primary}, and cleanup also failed: {cleanup_report}")]
    OperationFailedWithCleanup {
        primary: Box<ApplicationError>,
        cleanup_report: CleanupReport,
    },

    #[error(transparent)]
    Domain(#[from] domain::error::DomainError),

    #[error(transparent)]
    Port(#[from] ports::error::PortError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortErrorKind {
    Storage,
    Io,
    Network,
    NotFound,
    Conflict,
    Busy,
    InvalidStoredData,
    InvalidSource,
    ExternalToolFailed,
    Cancelled,
    Unsupported,
    Unexpected,
    AlreadyStopped,
}

impl PortErrorKind {
    pub fn from_error(err: &ports::error::PortError) -> Self {
        match err {
            ports::error::PortError::Storage { .. } => Self::Storage,
            ports::error::PortError::Io { .. } => Self::Io,
            ports::error::PortError::Network { .. } => Self::Network,
            ports::error::PortError::NotFound { .. } => Self::NotFound,
            ports::error::PortError::Conflict { .. } => Self::Conflict,
            ports::error::PortError::Busy { .. } => Self::Busy,
            ports::error::PortError::InvalidStoredData { .. } => Self::InvalidStoredData,
            ports::error::PortError::InvalidSource { .. } => Self::InvalidSource,
            ports::error::PortError::ExternalToolFailed { .. } => Self::ExternalToolFailed,
            ports::error::PortError::Cancelled => Self::Cancelled,
            ports::error::PortError::Unsupported { .. } => Self::Unsupported,
            ports::error::PortError::Unexpected { .. } => Self::Unexpected,
            ports::error::PortError::AlreadyStopped => Self::AlreadyStopped,
        }
    }
}

impl std::fmt::Display for PortErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CleanupTargetKind {
    Staging,
    Workspace,
}

impl std::fmt::Display for CleanupTargetKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct CleanupTarget {
    kind: CleanupTargetKind,
    key: String,
}

impl CleanupTarget {
    pub fn staging(key: impl Into<String>) -> Self {
        Self {
            kind: CleanupTargetKind::Staging,
            key: key.into(),
        }
    }

    pub fn workspace(key: impl Into<String>) -> Self {
        Self {
            kind: CleanupTargetKind::Workspace,
            key: key.into(),
        }
    }

    pub fn kind(&self) -> CleanupTargetKind {
        self.kind
    }

    pub fn key_ref(&self) -> &str {
        &self.key
    }
}

impl std::fmt::Display for CleanupTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl std::fmt::Debug for CleanupTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)
    }
}

pub struct CleanupFailure {
    target: CleanupTarget,
    error_kind: PortErrorKind,
    #[allow(dead_code)]
    raw_error: ports::error::PortError,
}

impl CleanupFailure {
    pub fn target_kind(&self) -> CleanupTargetKind {
        self.target.kind()
    }

    pub fn error_kind(&self) -> PortErrorKind {
        self.error_kind
    }
}

impl std::fmt::Display for CleanupFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (error_kind: {})", self.target, self.error_kind)
    }
}

impl std::fmt::Debug for CleanupFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CleanupFailure")
            .field("target", &self.target.kind())
            .field("error_kind", &self.error_kind)
            .finish()
    }
}

#[derive(Default)]
pub struct CleanupReport {
    failures: Vec<CleanupFailure>,
}

impl CleanupReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.failures.is_empty()
    }

    pub fn len(&self) -> usize {
        self.failures.len()
    }

    pub fn staging_failure_count(&self) -> usize {
        self.failures
            .iter()
            .filter(|f| f.target_kind() == CleanupTargetKind::Staging)
            .count()
    }

    pub fn workspace_failure_count(&self) -> usize {
        self.failures
            .iter()
            .filter(|f| f.target_kind() == CleanupTargetKind::Workspace)
            .count()
    }

    pub fn add_failure(&mut self, target: CleanupTarget, raw_error: ports::error::PortError) {
        let error_kind = PortErrorKind::from_error(&raw_error);
        self.failures.push(CleanupFailure {
            target,
            error_kind,
            raw_error,
        });
    }
}

impl std::fmt::Display for CleanupReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CleanupReport {{ staging_failures: {}, workspace_failures: {} }}",
            self.staging_failure_count(),
            self.workspace_failure_count()
        )
    }
}

impl std::fmt::Debug for CleanupReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CleanupReport")
            .field("staging_failures", &self.staging_failure_count())
            .field("workspace_failures", &self.workspace_failure_count())
            .finish()
    }
}
