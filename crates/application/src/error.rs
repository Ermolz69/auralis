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

    #[error(transparent)]
    Domain(#[from] domain::error::DomainError),

    #[error(transparent)]
    Port(#[from] ports::error::PortError),
}
