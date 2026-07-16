use application::error::ApplicationError;
use domain::error::DomainError;
use ports::error::PortError;
use serde::Serialize;

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(tag = "code", content = "message", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CommandError {
    NotFound(String),
    Validation(String),
    Conflict(String),
    Busy(String),
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
                PortError::Conflict { .. } => CommandError::Conflict(port_err.to_string()),
                PortError::Busy { .. } => CommandError::Busy(port_err.to_string()),
                PortError::InvalidSource { .. } => CommandError::Validation(port_err.to_string()),
                _ => CommandError::Repository(port_err.to_string()),
            },
            ApplicationError::OperationFailedWithCleanup { primary, cleanup_report } => {
                let mut dto = CommandError::from(*primary);
                // We could attach cleanup info to details if CommandError supported it,
                // but for now mapping to Internal with context is best.
                match &mut dto {
                    CommandError::Internal(msg) => {
                        *msg = format!("{} (Cleanup failed: {:?})", msg, cleanup_report);
                    }
                    _ => {
                        dto = CommandError::Internal(format!(
                            "Operation failed but mapped to non-internal error. Cleanup failed: {:?}",
                            cleanup_report
                        ));
                    }
                }
                dto
            }
            ApplicationError::Unexpected(msg) => CommandError::Internal(msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_error_serialization() {
        let not_found = CommandError::NotFound("project missing".into());
        let json = serde_json::to_string(&not_found).unwrap();
        assert_eq!(json, r#"{"code":"NOT_FOUND","message":"project missing"}"#);

        let conflict = CommandError::Conflict("conflict error".into());
        let json = serde_json::to_string(&conflict).unwrap();
        assert_eq!(json, r#"{"code":"CONFLICT","message":"conflict error"}"#);

        let busy = CommandError::Busy("database busy".into());
        let json = serde_json::to_string(&busy).unwrap();
        assert_eq!(json, r#"{"code":"BUSY","message":"database busy"}"#);
    }
}
