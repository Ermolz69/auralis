use common::wav::Pcm16Error;
use ports::PortError;

pub fn io(error: std::io::Error) -> PortError {
    PortError::Io {
        message: error.to_string(),
    }
}
pub fn invalid(message: impl Into<String>) -> PortError {
    PortError::InvalidSource {
        message: message.into(),
    }
}
pub fn external(message: impl Into<String>) -> PortError {
    PortError::ExternalToolFailed {
        tool: "ffmpeg".into(),
        message: message.into(),
    }
}
pub fn wave(error: Pcm16Error) -> PortError {
    match error {
        Pcm16Error::Io(error) => io(error),
        Pcm16Error::Invalid(message) => invalid(message),
    }
}
