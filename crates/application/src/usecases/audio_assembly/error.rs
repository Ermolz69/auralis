use ports::PortError;

pub(super) fn invalid(message: impl Into<String>) -> PortError {
    PortError::InvalidSource {
        message: message.into(),
    }
}
pub(super) fn conflict(message: impl Into<String>) -> PortError {
    PortError::Conflict {
        resource: "audio assembly".into(),
        message: message.into(),
    }
}
