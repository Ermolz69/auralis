use std::fmt;

#[derive(Debug)]
pub enum DatabaseTransitionError {
    UnknownSchema,
    CorruptDatabase(String),
    CheckpointBusy,
    BackupFailed(String),
    BackupValidationFailed(String),
    FreshDatabaseCreationFailed(String),
    IncompleteTransition,
    InspectionFailed(String),
    TransitionLocked,
    TransitionRecoveryFailed(String),
    NewDatabaseValidationFailed(String),
}

impl fmt::Display for DatabaseTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DatabaseTransitionError::UnknownSchema => {
                write!(f, "Unknown database schema detected. Refusing to operate.")
            }
            DatabaseTransitionError::CorruptDatabase(err) => {
                write!(f, "Database is corrupt or not a database: {}", err)
            }
            DatabaseTransitionError::CheckpointBusy => {
                write!(f, "Database checkpoint is busy. Cannot transition safely.")
            }
            DatabaseTransitionError::BackupFailed(err) => {
                write!(f, "Failed to backup legacy database: {}", err)
            }
            DatabaseTransitionError::BackupValidationFailed(err) => {
                write!(f, "Legacy database backup validation failed: {}", err)
            }
            DatabaseTransitionError::FreshDatabaseCreationFailed(err) => {
                write!(f, "Failed to create new managed database: {}", err)
            }
            DatabaseTransitionError::IncompleteTransition => {
                write!(f, "Database transition is incomplete.")
            }
            DatabaseTransitionError::InspectionFailed(err) => {
                write!(f, "Failed to inspect database schema: {}", err)
            }
            DatabaseTransitionError::TransitionLocked => {
                write!(f, "Transition lock is currently held by another process.")
            }
            DatabaseTransitionError::TransitionRecoveryFailed(err) => {
                write!(
                    f,
                    "Failed to recover from an interrupted transition: {}",
                    err
                )
            }
            DatabaseTransitionError::NewDatabaseValidationFailed(err) => {
                write!(f, "New database validation failed before switch: {}", err)
            }
        }
    }
}

impl std::error::Error for DatabaseTransitionError {}

impl From<DatabaseTransitionError> for ports::error::PortError {
    fn from(err: DatabaseTransitionError) -> Self {
        ports::error::PortError::Unexpected {
            message: err.to_string(),
        }
    }
}
