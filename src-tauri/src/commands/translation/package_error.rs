use crate::dto::error::CommandError;
use adapters_translate::TranslationPackageError;

pub(super) fn map(error: TranslationPackageError) -> CommandError {
    match error {
        TranslationPackageError::Invalid(_) => {
            CommandError::Validation("Model package is invalid or unavailable".into())
        }
        TranslationPackageError::Busy => {
            CommandError::Busy("A model installation is already running".into())
        }
        TranslationPackageError::Download(_) => {
            CommandError::Repository("Model download failed".into())
        }
        TranslationPackageError::Install(_) => {
            CommandError::Repository("Model installation failed".into())
        }
        TranslationPackageError::Runtime(error) => CommandError::from(error),
        TranslationPackageError::Io(_) => CommandError::Io("Model package storage failed".into()),
        TranslationPackageError::Manifest(_) | TranslationPackageError::Json(_) => {
            CommandError::Internal("Model package metadata is invalid".into())
        }
    }
}
