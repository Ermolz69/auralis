use ports::PortError;

pub(crate) fn invalid(message: impl Into<String>) -> PortError {
    PortError::InvalidSource {
        message: message.into(),
    }
}
pub(crate) fn conflict(message: impl Into<String>) -> PortError {
    PortError::Conflict {
        resource: "audio assembly".into(),
        message: message.into(),
    }
}
pub(crate) fn database(error: sqlx::Error) -> PortError {
    if error
        .as_database_error()
        .is_some_and(|error| error.is_unique_violation())
    {
        conflict("Assembly intent, attempt, artifact or job identity already exists")
    } else {
        crate::sqlite::helpers::map_sqlite_error("audio_assembly", error)
    }
}
pub(crate) fn integer(value: u64) -> Result<i64, PortError> {
    i64::try_from(value).map_err(|_| invalid("Assembly integer exceeds SQLite range"))
}
pub(crate) fn uuid(value: &str) -> Result<uuid::Uuid, PortError> {
    uuid::Uuid::parse_str(value).map_err(|_| invalid("Stored assembly identifier is invalid"))
}
