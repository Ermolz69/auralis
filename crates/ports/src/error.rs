#[derive(Debug, thiserror::Error)]
pub enum PortError {
    #[error("I/O error: {message}")]
    Io { message: String },

    #[error("Network error: {message}")]
    Network { message: String },

    #[error("Not found: {resource}")]
    NotFound { resource: String },

    #[error("Invalid source: {message}")]
    InvalidSource { message: String },

    #[error("External tool failed: {tool}: {message}")]
    ExternalToolFailed { tool: String, message: String },

    #[error("Operation cancelled")]
    Cancelled,

    #[error("Unsupported operation: {message}")]
    Unsupported { message: String },

    #[error("Unexpected port error: {message}")]
    Unexpected { message: String },
}
