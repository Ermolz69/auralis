use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ObservabilityValidationError {
    #[error("max_log_files must be greater than 0")]
    ZeroMaxLogFiles,
    #[error("buffer_capacity must be greater than 0")]
    ZeroBufferCapacity,
    #[error("default tracing filter is invalid")]
    InvalidDefaultFilter,
}
