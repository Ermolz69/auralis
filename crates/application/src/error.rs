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

    #[error(transparent)]
    Domain(#[from] domain::error::DomainError),

    #[error(transparent)]
    Port(#[from] ports::error::PortError),
}
