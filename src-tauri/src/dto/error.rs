#![allow(clippy::unwrap_used, clippy::expect_used)]
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
    Cancelled(String),
}

fn format_cleanup_report(report: &application::error::CleanupReport) -> String {
    format!(
        "An unexpected internal error occurred (Cleanup failed: {} staging cleanup failures, {} workspace cleanup failures)",
        report.staging_failure_count(),
        report.workspace_failure_count()
    )
}

impl From<ApplicationError> for CommandError {
    fn from(err: ApplicationError) -> Self {
        match err {
            ApplicationError::ProjectNotFound(_) => {
                CommandError::NotFound("Project not found".to_string())
            }
            ApplicationError::JobNotFound(_) => CommandError::NotFound("Job not found".to_string()),
            ApplicationError::InvalidOperation { .. } => {
                CommandError::Validation("Invalid operation".to_string())
            }
            ApplicationError::PipelineStartFailed { .. }
            | ApplicationError::PipelineStartFailedNeedsRecovery { .. }
            | ApplicationError::Unexpected(_)
            | ApplicationError::Configuration(_) => {
                CommandError::Internal("An unexpected internal error occurred".to_string())
            }
            ApplicationError::Domain(domain_err) => match domain_err {
                DomainError::ValidationError(_) => {
                    CommandError::Validation("Validation failed".to_string())
                }
                DomainError::InvalidStateTransition { .. } => {
                    CommandError::Validation("Invalid state transition".to_string())
                }
                DomainError::StateOverflow(_) => {
                    CommandError::Validation("State overflow".to_string())
                }
            },
            ApplicationError::Port(port_err) => match port_err {
                PortError::NotFound { .. } => {
                    CommandError::NotFound("Resource not found".to_string())
                }
                PortError::Conflict { .. } => {
                    CommandError::Conflict("A concurrency conflict occurred".to_string())
                }
                PortError::Busy { .. } => {
                    CommandError::Busy("The system is busy, please try again".to_string())
                }
                PortError::InvalidSource { .. } => {
                    CommandError::Validation("Invalid input source".to_string())
                }
                PortError::Unsupported { .. } => {
                    CommandError::Validation("Operation not supported".to_string())
                }
                PortError::Cancelled => {
                    CommandError::Cancelled("The operation was cancelled".to_string())
                }
                PortError::AlreadyStopped => {
                    CommandError::Internal("An unexpected internal error occurred".to_string())
                }
                PortError::Storage { .. }
                | PortError::Io { .. }
                | PortError::Network { .. }
                | PortError::InvalidStoredData { .. }
                | PortError::ExternalToolFailed { .. }
                | PortError::Unexpected { .. } => {
                    CommandError::Repository("A repository error occurred".to_string())
                }
            },
            ApplicationError::OperationFailedWithCleanup { cleanup_report, .. } => {
                CommandError::Internal(format_cleanup_report(&cleanup_report))
            }
        }
    }
}

impl From<PortError> for CommandError {
    fn from(err: PortError) -> Self {
        CommandError::from(ApplicationError::Port(err))
    }
}

pub fn parse_project_id(id_str: &str) -> Result<domain::project::ProjectId, CommandError> {
    id_str
        .parse()
        .map_err(|_| CommandError::Validation("Invalid identifier".to_string()))
}

pub fn parse_job_id(id_str: &str) -> Result<domain::job::JobId, CommandError> {
    id_str
        .parse()
        .map_err(|_| CommandError::Validation("Invalid identifier".to_string()))
}

pub fn parse_artifact_id(id_str: &str) -> Result<domain::media::ArtifactId, CommandError> {
    id_str
        .parse()
        .map_err(|_| CommandError::Validation("Invalid identifier".to_string()))
}

pub fn parse_artifact_kind(kind_str: &str) -> Result<domain::media::ArtifactKind, CommandError> {
    serde_json::from_str(&format!("\"{}\"", kind_str))
        .map_err(|_| CommandError::Validation("Invalid artifact kind".to_string()))
}

pub fn map_job_dto_result<T, E>(res: Result<T, E>) -> Result<T, CommandError> {
    res.map_err(|_| CommandError::Internal("An unexpected internal error occurred".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use application::error::CleanupReport;
    use domain::job::JobId;
    use domain::project::ProjectId;

    #[test]
    fn test_helper_parsing_and_mapping() {
        assert!(parse_project_id("invalid-uuid").is_err());
        assert_eq!(
            parse_project_id("invalid-uuid").unwrap_err(),
            CommandError::Validation("Invalid identifier".to_string())
        );
        assert_eq!(
            parse_job_id("invalid-uuid").unwrap_err(),
            CommandError::Validation("Invalid identifier".to_string())
        );
        assert_eq!(
            parse_artifact_id("invalid-uuid").unwrap_err(),
            CommandError::Validation("Invalid identifier".to_string())
        );

        assert_eq!(
            parse_artifact_kind("invalid-kind").unwrap_err(),
            CommandError::Validation("Invalid artifact kind".to_string())
        );

        let mapping_err = map_job_dto_result::<(), _>(Err("err"));
        assert_eq!(
            mapping_err.unwrap_err(),
            CommandError::Internal("An unexpected internal error occurred".to_string())
        );
    }

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

        let cancelled = CommandError::Cancelled("cancelled".into());
        let json = serde_json::to_string(&cancelled).unwrap();
        assert_eq!(json, r#"{"code":"CANCELLED","message":"cancelled"}"#);
    }

    #[test]
    fn test_matrix_mapping_correctness() {
        // NotFound
        assert_eq!(
            CommandError::from(ApplicationError::ProjectNotFound(ProjectId::new())),
            CommandError::NotFound("Project not found".into())
        );
        assert_eq!(
            CommandError::from(ApplicationError::JobNotFound(JobId::new())),
            CommandError::NotFound("Job not found".into())
        );
        assert_eq!(
            CommandError::from(ApplicationError::Port(PortError::NotFound {
                resource: "key".into()
            })),
            CommandError::NotFound("Resource not found".into())
        );

        // Validation
        assert_eq!(
            CommandError::from(ApplicationError::Domain(DomainError::ValidationError(
                "bad".into()
            ))),
            CommandError::Validation("Validation failed".into())
        );
        assert_eq!(
            CommandError::from(ApplicationError::Domain(
                DomainError::InvalidStateTransition {
                    from: "a".into(),
                    to: "b".into()
                }
            )),
            CommandError::Validation("Invalid state transition".into())
        );
        assert_eq!(
            CommandError::from(ApplicationError::InvalidOperation {
                message: "bad".into()
            }),
            CommandError::Validation("Invalid operation".into())
        );

        // Conflict / Busy
        assert_eq!(
            CommandError::from(ApplicationError::Port(PortError::Conflict {
                resource: "x".into(),
                message: "y".into()
            })),
            CommandError::Conflict("A concurrency conflict occurred".into())
        );
        assert_eq!(
            CommandError::from(ApplicationError::Port(PortError::Busy {
                message: "y".into()
            })),
            CommandError::Busy("The system is busy, please try again".into())
        );

        // Cancelled
        assert_eq!(
            CommandError::from(ApplicationError::Port(PortError::Cancelled)),
            CommandError::Cancelled("The operation was cancelled".into())
        );
    }

    #[test]
    fn test_cleanup_report_formatting() {
        let mut report = CleanupReport::new();
        // Add 2 staging failures
        report.add_failure(
            application::error::CleanupTarget::staging("staging/proj/file.mp4"),
            PortError::Io {
                message: "staging fail".into(),
            },
        );
        report.add_failure(
            application::error::CleanupTarget::staging("staging/proj/file2.mp4"),
            PortError::Io {
                message: "staging fail".into(),
            },
        );
        // Add 1 workspace failure
        report.add_failure(
            application::error::CleanupTarget::workspace("tmp/1/subs"),
            PortError::Io {
                message: "workspace fail".into(),
            },
        );

        let err = ApplicationError::OperationFailedWithCleanup {
            primary: Box::new(ApplicationError::Unexpected("unexpected".into())),
            cleanup_report: report,
        };

        let cmd_err = CommandError::from(err);
        if let CommandError::Internal(msg) = cmd_err {
            assert_eq!(
                msg,
                "An unexpected internal error occurred (Cleanup failed: 2 staging cleanup failures, 1 workspace cleanup failures)"
            );
        } else {
            panic!("Expected CommandError::Internal");
        }
    }

    #[test]
    fn test_sensitive_sentinel_json_serialization() {
        // Create an error containing all sensitive fixtures at once
        let sensitive_msg = "sqlx::Error::Database(C:\\Users\\secret\\video.mp4 token=SECRET Bearer sct_token SELECT * FROM projects) ffmpeg: stderr: failed to encode /var/log/app.log \\\\server\\share\\file.txt https://user:pass@example.com/path?token=abc#frag password=secretpwd Bearer sct_token \"This is a transcript text payload.\" {\"title\":\"test\"} staging/project1/file.mp4";

        let err = ApplicationError::Unexpected(sensitive_msg.to_string());
        let cmd_err = CommandError::from(err);
        let json = serde_json::to_string(&cmd_err).unwrap();

        // The serialized output must be generic and contain NONE of the sensitive items
        assert!(json.contains("INTERNAL"));
        assert!(json.contains("An unexpected internal error occurred"));
        assert!(!json.contains("secret"));
        assert!(!json.contains("SECRET"));
        assert!(!json.contains("token"));
        assert!(!json.contains("Bearer"));
        assert!(!json.contains("sct_token"));
        assert!(!json.contains("sqlx"));
        assert!(!json.contains("SELECT"));
        assert!(!json.contains("ffmpeg"));
        assert!(!json.contains("stderr"));
        assert!(!json.contains("log"));
        assert!(!json.contains("server"));
        assert!(!json.contains("pass"));
        assert!(!json.contains("password"));
        assert!(!json.contains("transcript"));
        assert!(!json.contains("title"));
        assert!(!json.contains("staging"));
    }
}
