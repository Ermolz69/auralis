use domain::error::DomainError;
use domain::job::{JobStatus, TerminalOutcome};
use ports::error::PortError;

pub enum ApplyRecoveryActionError {
    Domain,
    InvalidRecoveryAction(&'static str),
    Persistence(PortError),
}

impl ApplyRecoveryActionError {
    pub fn error_type(&self) -> &'static str {
        match self {
            Self::Domain => "DomainError",
            Self::InvalidRecoveryAction(_) => "InvalidRecoveryAction",
            Self::Persistence(error) => match error {
                PortError::Storage { .. } => "Storage",
                PortError::Io { .. } => "Io",
                PortError::Network { .. } => "Network",
                PortError::NotFound { .. } => "NotFound",
                PortError::Conflict { .. } => "Conflict",
                PortError::Busy { .. } => "Busy",
                PortError::InvalidStoredData { .. } => "InvalidStoredData",
                PortError::InvalidSource { .. } => "InvalidSource",
                PortError::ExternalToolFailed { .. } => "ExternalToolFailed",
                PortError::Cancelled => "Cancelled",
                PortError::Unsupported { .. } => "Unsupported",
                PortError::AlreadyStopped => "AlreadyStopped",
                PortError::Unexpected { .. } => "Unexpected",
            },
        }
    }

    pub fn safe_message(&self) -> &'static str {
        match self {
            Self::Domain => "Recovery domain transition failed",
            Self::InvalidRecoveryAction(message) => message,
            Self::Persistence(_) => "Recovery persistence operation failed",
        }
    }
}

impl From<DomainError> for ApplyRecoveryActionError {
    fn from(_: DomainError) -> Self {
        Self::Domain
    }
}

impl From<PortError> for ApplyRecoveryActionError {
    fn from(value: PortError) -> Self {
        Self::Persistence(value)
    }
}

pub fn terminal_outcome_for_status(
    status: &JobStatus,
) -> Result<TerminalOutcome, ApplyRecoveryActionError> {
    match status {
        JobStatus::Completed => Ok(TerminalOutcome::Completed),
        JobStatus::Failed => Ok(TerminalOutcome::Failed),
        JobStatus::Cancelled => Ok(TerminalOutcome::Cancelled),
        JobStatus::Pending | JobStatus::Running => {
            Err(ApplyRecoveryActionError::InvalidRecoveryAction(
                "Recovery action expected a terminal job status",
            ))
        }
    }
}
