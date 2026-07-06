use thiserror::Error;
use crate::id::JobId;

#[derive(Debug, Error)]
pub enum JobError {
    #[error("Job not found: {0}")]
    NotFound(JobId),
    
    #[error("Invalid state transition")]
    InvalidState,

    #[error("Internal error: {0}")]
    Internal(String),
}
