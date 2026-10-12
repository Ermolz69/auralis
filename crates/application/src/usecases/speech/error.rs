use ports::error::PortError;
pub(super) fn invalid(message: impl Into<String>) -> PortError {
    PortError::InvalidSource {
        message: message.into(),
    }
}
pub(super) fn conflict(message: &str) -> PortError {
    PortError::Conflict {
        resource: "speech".into(),
        message: message.into(),
    }
}
