use crate::RuntimeShutdownReport;

#[derive(Debug, thiserror::Error)]
pub enum AppRunError {
    #[error("application configuration is invalid")]
    Configuration(#[source] application::error::ApplicationError),

    #[error("observability configuration is invalid")]
    Observability(#[from] crate::observability::error::ObservabilityValidationError),

    #[error("failed to build Tauri application")]
    TauriBuild(#[source] tauri::Error),

    #[error("runtime shutdown was not graceful")]
    Shutdown(RuntimeShutdownReport),

    #[error("runtime shutdown event was not observed")]
    ShutdownNotObserved,
}

impl AppRunError {
    pub fn diagnostic(&self) -> crate::observability::diagnostic::ProcessDiagnostic {
        use crate::observability::diagnostic::{
            DiagnosticKind, DiagnosticLevel, ProcessDiagnostic,
        };
        let kind = match self {
            AppRunError::Configuration(_) => DiagnosticKind::ApplicationConfigurationInvalid,
            AppRunError::Observability(_) => DiagnosticKind::ObservabilityConfigurationInvalid,
            AppRunError::TauriBuild(_) => DiagnosticKind::TauriBuildFailed,
            AppRunError::Shutdown(_) => DiagnosticKind::ShutdownFailed,
            AppRunError::ShutdownNotObserved => DiagnosticKind::ShutdownNotObserved,
        };
        ProcessDiagnostic {
            level: DiagnosticLevel::Error,
            kind,
            os_code: None,
            count: None,
            fallback: None,
        }
    }
}
