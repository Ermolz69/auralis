use application::error::ApplicationError;
use domain::error::DomainError;
use ports::error::PortError;
use serde::Serialize;

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommandError {
    NotFound(String),
    Validation(String),
    Repository(String),
    Internal(String),
}

impl From<ApplicationError> for CommandError {
    fn from(err: ApplicationError) -> Self {
        match err {
            ApplicationError::ProjectNotFound(_) | ApplicationError::JobNotFound(_) => {
                CommandError::NotFound(err.to_string())
            }
            ApplicationError::InvalidOperation { .. } => CommandError::Validation(err.to_string()),
            ApplicationError::PipelineStartFailed { scheduling_error } => {
                CommandError::Internal(format!("Failed to start pipeline: scheduling failed ({})", scheduling_error))
            }
            ApplicationError::PipelineStartFailedNeedsRecovery { scheduling_error, compensation_error } => {
                CommandError::Internal(format!("Failed to start pipeline: scheduling failed ({}) AND compensation failed ({}) - RECOVERY REQUIRED", scheduling_error, compensation_error))
            }
            ApplicationError::Domain(domain_err) => match domain_err {
                DomainError::ValidationError(_) | DomainError::InvalidStateTransition { .. } => {
                    CommandError::Validation(domain_err.to_string())
                }
            },
            ApplicationError::Port(port_err) => match port_err {
                PortError::NotFound { .. } => CommandError::NotFound(port_err.to_string()),
                PortError::InvalidSource { .. } => CommandError::Validation(port_err.to_string()),
                _ => CommandError::Repository(port_err.to_string()),
            },
        }
    }
}
