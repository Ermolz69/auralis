#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("State overflow error: {0}")]
    StateOverflow(String),
}
