use super::sink::OwnedSink;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TracingMode {
    FileAndConsole,
    ConsoleOnly,
    ExistingSubscriber,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubscriberInstallError {
    AlreadyInstalled,
}

#[derive(Debug)]
pub enum ObservabilityResourceError {
    Io { os_code: Option<i32> },
    AppenderInitialization,
}

pub(crate) trait ObservabilityEnvironment: Send + Sync {
    fn has_subscriber(&self) -> bool;
    fn create_directory(&self, path: &std::path::Path) -> Result<(), ObservabilityResourceError>;
    fn build_non_blocking(
        &self,
        path: &std::path::Path,
        rotation: tracing_appender::rolling::Rotation,
        max_log_files: usize,
        capacity: usize,
        lossy: bool,
    ) -> Result<OwnedSink, ObservabilityResourceError>;
    fn build_console(&self) -> OwnedSink {
        super::diagnostic::StderrDiagnosticSink::owned().1
    }
    fn set_global_default(&self, dispatch: tracing::Dispatch)
    -> Result<(), SubscriberInstallError>;
}

pub struct ResolvedEnvFilter {
    pub filter: EnvFilter,
    pub used_default: bool,
}

pub fn resolve_env_filter(
    raw_value: Option<&str>,
    default_filter: &EnvFilter,
) -> ResolvedEnvFilter {
    match raw_value {
        Some(val) => match EnvFilter::try_new(val) {
            Ok(filter) => ResolvedEnvFilter {
                filter,
                used_default: false,
            },
            Err(_) => ResolvedEnvFilter {
                filter: default_filter.clone(),
                used_default: true,
            },
        },
        None => ResolvedEnvFilter {
            filter: default_filter.clone(),
            used_default: false,
        },
    }
}

use std::sync::atomic::{AtomicBool, Ordering};
static SUBSCRIBER_INSTALLED: AtomicBool = AtomicBool::new(false);

pub(crate) struct RealObservabilityEnvironment;

impl ObservabilityEnvironment for RealObservabilityEnvironment {
    fn has_subscriber(&self) -> bool {
        SUBSCRIBER_INSTALLED.load(Ordering::Acquire)
    }

    fn create_directory(&self, path: &std::path::Path) -> Result<(), ObservabilityResourceError> {
        std::fs::create_dir_all(path).map_err(|e| ObservabilityResourceError::Io {
            os_code: e.raw_os_error(),
        })
    }

    fn build_non_blocking(
        &self,
        path: &std::path::Path,
        rotation: tracing_appender::rolling::Rotation,
        max_log_files: usize,
        capacity: usize,
        lossy: bool,
    ) -> Result<OwnedSink, ObservabilityResourceError> {
        let appender = super::file_budget::FileBudget::new(path, rotation, max_log_files).map_err(
            |error| ObservabilityResourceError::Io {
                os_code: error.raw_os_error(),
            },
        )?;
        Ok(OwnedSink::new(appender, capacity, lossy))
    }

    fn set_global_default(
        &self,
        dispatch: tracing::Dispatch,
    ) -> Result<(), SubscriberInstallError> {
        tracing::dispatcher::set_global_default(dispatch)
            .map(|_| {
                SUBSCRIBER_INSTALLED.store(true, Ordering::Release);
            })
            .map_err(|_| SubscriberInstallError::AlreadyInstalled)
    }
}
