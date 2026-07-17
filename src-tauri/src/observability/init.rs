use super::config::{LogDestination, LogFormat, ObservabilityConfig, OverflowPolicy};
use super::diagnostic::{
    DiagnosticFallback, DiagnosticKind, DiagnosticLevel, DiagnosticSink, ProcessDiagnostic,
};
use crate::TracingShutdownOutcome;
use std::sync::Arc;
use tracing_appender::non_blocking::{ErrorCounter, WorkerGuard};
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter, Layer};

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
    ) -> Result<
        (
            tracing_appender::non_blocking::NonBlocking,
            tracing_appender::non_blocking::WorkerGuard,
        ),
        ObservabilityResourceError,
    >;
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

pub struct TracingGuard {
    worker_guard: Option<WorkerGuard>,
    pub active_mode: TracingMode,
    error_counter: Option<ErrorCounter>,
    sink: Arc<dyn DiagnosticSink>,
}

impl TracingGuard {
    pub fn shutdown(mut self, timeout: std::time::Duration) -> TracingShutdownOutcome {
        if self.active_mode == TracingMode::ExistingSubscriber {
            return TracingShutdownOutcome::NotOwned;
        }

        if let Some(ref counter) = self.error_counter {
            let dropped = counter.dropped_lines();
            if dropped > 0 {
                tracing::warn!(
                    dropped_lines = dropped,
                    action = "tracing_buffer_overflow",
                    "tracing events were dropped"
                );
            }
        }

        if let Some(guard) = self.worker_guard.take() {
            let (tx, rx) = std::sync::mpsc::channel();
            let builder = std::thread::Builder::new().name("tracing-flush".to_string());
            match builder.spawn(move || {
                drop(guard);
                let _ = tx.send(());
            }) {
                Ok(_) => match rx.recv_timeout(timeout) {
                    Ok(_) => TracingShutdownOutcome::Flushed,
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        self.sink.emit(ProcessDiagnostic {
                            level: DiagnosticLevel::Warning,
                            kind: DiagnosticKind::TracingFlushTimedOut,
                            os_code: None,
                            count: None,
                            fallback: None,
                        });
                        TracingShutdownOutcome::TimedOut
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        TracingShutdownOutcome::Flushed
                    }
                },
                Err(_) => TracingShutdownOutcome::FlushThreadStartFailed,
            }
        } else {
            TracingShutdownOutcome::Flushed
        }
    }
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        if let Some(ref counter) = self.error_counter {
            let dropped = counter.dropped_lines();
            if dropped > 0 {
                self.sink.emit(ProcessDiagnostic {
                    level: DiagnosticLevel::Warning,
                    kind: DiagnosticKind::BufferOverflow,
                    os_code: None,
                    count: Some(dropped as u64),
                    fallback: None,
                });
            }
        }
        if let Some(guard) = self.worker_guard.take() {
            drop(guard);
        }
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
    ) -> Result<
        (
            tracing_appender::non_blocking::NonBlocking,
            tracing_appender::non_blocking::WorkerGuard,
        ),
        ObservabilityResourceError,
    > {
        let appender = tracing_appender::rolling::Builder::new()
            .rotation(rotation)
            .filename_prefix("auralis.log")
            .max_log_files(max_log_files)
            .build(path)
            .map_err(|_| ObservabilityResourceError::AppenderInitialization)?;

        let (non_blocking, guard) = tracing_appender::non_blocking::NonBlockingBuilder::default()
            .lossy(lossy)
            .buffered_lines_limit(capacity)
            .finish(appender);

        Ok((non_blocking, guard))
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

pub fn init(config: ObservabilityConfig, sink: Arc<dyn DiagnosticSink>) -> TracingGuard {
    init_with_environment(config, sink, &RealObservabilityEnvironment)
}

pub(crate) fn init_with_environment(
    config: ObservabilityConfig,
    sink: Arc<dyn DiagnosticSink>,
    env: &dyn ObservabilityEnvironment,
) -> TracingGuard {
    if env.has_subscriber() {
        sink.emit(ProcessDiagnostic {
            level: DiagnosticLevel::Warning,
            kind: DiagnosticKind::SubscriberAlreadyInstalled,
            os_code: None,
            count: None,
            fallback: Some(DiagnosticFallback::ExistingSubscriber),
        });
        return TracingGuard {
            worker_guard: None,
            active_mode: TracingMode::ExistingSubscriber,
            error_counter: None,
            sink,
        };
    }

    let mut use_console_only = false;
    let mut file_layer = None;
    let mut guard = None;
    let mut error_counter = None;

    match config.log_dir {
        LogDestination::Disabled => {
            use_console_only = true;
        }
        LogDestination::Unavailable(_) => {
            sink.emit(ProcessDiagnostic {
                level: DiagnosticLevel::Warning,
                kind: DiagnosticKind::LogDirectoryUnavailable,
                os_code: None,
                count: None,
                fallback: Some(DiagnosticFallback::ConsoleOnly),
            });
            use_console_only = true;
        }
        LogDestination::Directory(log_dir) => match env.create_directory(&log_dir) {
            Err(e) => {
                let os_code = match e {
                    ObservabilityResourceError::Io { os_code } => os_code,
                    _ => None,
                };
                sink.emit(ProcessDiagnostic {
                    level: DiagnosticLevel::Warning,
                    kind: DiagnosticKind::LogDirectoryUnavailable,
                    os_code,
                    count: None,
                    fallback: Some(DiagnosticFallback::ConsoleOnly),
                });
                use_console_only = true;
            }
            Ok(()) => {
                let is_lossy = config.settings.overflow_policy == OverflowPolicy::DropWhenFull;
                match env.build_non_blocking(
                    &log_dir,
                    config.settings.rotation.clone(),
                    config.settings.max_log_files,
                    config.settings.buffer_capacity,
                    is_lossy,
                ) {
                    Ok((non_blocking, worker_guard)) => {
                        if is_lossy {
                            error_counter = Some(non_blocking.error_counter().clone());
                        }
                        guard = Some(worker_guard);

                        let file_base = fmt::layer().with_writer(non_blocking).with_ansi(false);
                        let fl = match config.settings.file_format {
                            LogFormat::Pretty => file_base.pretty().boxed(),
                            LogFormat::Compact => file_base.compact().boxed(),
                            LogFormat::Json => file_base.json().boxed(),
                        };
                        file_layer = Some(fl);
                    }
                    Err(e) => {
                        let os_code = match e {
                            ObservabilityResourceError::Io { os_code } => os_code,
                            _ => None,
                        };
                        sink.emit(ProcessDiagnostic {
                            level: DiagnosticLevel::Warning,
                            kind: DiagnosticKind::FileAppenderUnavailable,
                            os_code,
                            count: None,
                            fallback: Some(DiagnosticFallback::ConsoleOnly),
                        });
                        use_console_only = true;
                    }
                }
            }
        },
    }

    let active_mode = if use_console_only {
        TracingMode::ConsoleOnly
    } else {
        TracingMode::FileAndConsole
    };

    let raw_rust_log = std::env::var("RUST_LOG").ok();
    let resolved = resolve_env_filter(raw_rust_log.as_deref(), &config.settings.default_filter);
    if resolved.used_default {
        sink.emit(ProcessDiagnostic {
            level: DiagnosticLevel::Warning,
            kind: DiagnosticKind::InvalidRustLog,
            os_code: None,
            count: None,
            fallback: None,
        });
    }

    let console_base = fmt::layer()
        .with_writer(crate::observability::diagnostic::stderr_writer)
        .with_ansi(true);
    let console_layer = match config.settings.console_format {
        LogFormat::Pretty => console_base.pretty().boxed(),
        LogFormat::Compact => console_base.compact().boxed(),
        LogFormat::Json => console_base.json().boxed(),
    };

    let registry = tracing_subscriber::registry()
        .with(resolved.filter)
        .with(console_layer);

    let dispatch = if let Some(fl) = file_layer {
        tracing::Dispatch::new(registry.with(fl))
    } else {
        tracing::Dispatch::new(registry)
    };

    match env.set_global_default(dispatch) {
        Ok(()) => TracingGuard {
            worker_guard: guard,
            active_mode,
            error_counter,
            sink,
        },
        Err(SubscriberInstallError::AlreadyInstalled) => {
            sink.emit(ProcessDiagnostic {
                level: DiagnosticLevel::Warning,
                kind: DiagnosticKind::SubscriberAlreadyInstalled,
                os_code: None,
                count: None,
                fallback: Some(DiagnosticFallback::ExistingSubscriber),
            });
            // Drop our created worker guard boundedly immediately
            drop(guard);
            TracingGuard {
                worker_guard: None,
                active_mode: TracingMode::ExistingSubscriber,
                error_counter: None,
                sink,
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::super::config::{ObservabilitySettings, ValidatedObservabilitySettings};
    use super::*;
    use std::io::Write;
    use std::sync::Mutex;

    struct MockSink {
        emitted: Arc<Mutex<Vec<ProcessDiagnostic>>>,
    }

    impl DiagnosticSink for MockSink {
        fn emit(&self, diag: ProcessDiagnostic) {
            self.emitted.lock().unwrap().push(diag);
        }
    }

    struct MockEnv {
        has_sub: bool,
        dir_res: Result<(), ObservabilityResourceError>,
        appender_res: Mutex<
            Option<
                Result<
                    (
                        tracing_appender::non_blocking::NonBlocking,
                        tracing_appender::non_blocking::WorkerGuard,
                    ),
                    ObservabilityResourceError,
                >,
            >,
        >,
        set_res: Result<(), SubscriberInstallError>,
    }

    impl ObservabilityEnvironment for MockEnv {
        fn has_subscriber(&self) -> bool {
            self.has_sub
        }
        fn create_directory(
            &self,
            _path: &std::path::Path,
        ) -> Result<(), ObservabilityResourceError> {
            match &self.dir_res {
                Ok(_) => Ok(()),
                Err(e) => Err(match e {
                    ObservabilityResourceError::Io { os_code } => {
                        ObservabilityResourceError::Io { os_code: *os_code }
                    }
                    ObservabilityResourceError::AppenderInitialization => {
                        ObservabilityResourceError::AppenderInitialization
                    }
                }),
            }
        }
        fn build_non_blocking(
            &self,
            _path: &std::path::Path,
            _rotation: tracing_appender::rolling::Rotation,
            _max_log_files: usize,
            _capacity: usize,
            _lossy: bool,
        ) -> Result<
            (
                tracing_appender::non_blocking::NonBlocking,
                tracing_appender::non_blocking::WorkerGuard,
            ),
            ObservabilityResourceError,
        > {
            self.appender_res.lock().unwrap().take().unwrap()
        }
        fn set_global_default(
            &self,
            _dispatch: tracing::Dispatch,
        ) -> Result<(), SubscriberInstallError> {
            self.set_res
        }
    }

    #[test]
    fn test_resolve_env_filter_pure() {
        let default_filter = EnvFilter::new("info");

        let res_ok = resolve_env_filter(Some("debug"), &default_filter);
        assert!(!res_ok.used_default);

        let res_invalid = resolve_env_filter(Some("!!!invalid!!!"), &default_filter);
        assert!(res_invalid.used_default);

        let res_none = resolve_env_filter(None, &default_filter);
        assert!(!res_none.used_default);
    }

    #[test]
    fn test_init_disabled() {
        let emitted = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::new(MockSink {
            emitted: emitted.clone(),
        });
        let settings = ObservabilitySettings::for_build(true);
        let config = ObservabilityConfig {
            settings: ValidatedObservabilitySettings::try_from(settings).unwrap(),
            log_dir: LogDestination::Disabled,
        };
        let env = MockEnv {
            has_sub: false,
            dir_res: Ok(()),
            appender_res: Mutex::new(None),
            set_res: Ok(()),
        };
        let guard = init_with_environment(config, sink, &env);
        assert_eq!(guard.active_mode, TracingMode::ConsoleOnly);
        assert!(emitted.lock().unwrap().is_empty());
    }

    #[test]
    fn test_init_dir_failure() {
        let emitted = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::new(MockSink {
            emitted: emitted.clone(),
        });
        let settings = ObservabilitySettings::for_build(true);
        let config = ObservabilityConfig {
            settings: ValidatedObservabilitySettings::try_from(settings).unwrap(),
            log_dir: LogDestination::Directory(std::path::PathBuf::from("/dummy")),
        };
        let env = MockEnv {
            has_sub: false,
            dir_res: Err(ObservabilityResourceError::Io { os_code: Some(5) }),
            appender_res: Mutex::new(None),
            set_res: Ok(()),
        };
        let guard = init_with_environment(config, sink, &env);
        assert_eq!(guard.active_mode, TracingMode::ConsoleOnly);
        let diags = emitted.lock().unwrap();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].kind, DiagnosticKind::LogDirectoryUnavailable);
        assert_eq!(diags[0].os_code, Some(5));
    }

    #[test]
    fn test_init_appender_failure() {
        let emitted = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::new(MockSink {
            emitted: emitted.clone(),
        });
        let settings = ObservabilitySettings::for_build(true);
        let config = ObservabilityConfig {
            settings: ValidatedObservabilitySettings::try_from(settings).unwrap(),
            log_dir: LogDestination::Directory(std::path::PathBuf::from("/dummy")),
        };
        let env = MockEnv {
            has_sub: false,
            dir_res: Ok(()),
            appender_res: Mutex::new(Some(Err(ObservabilityResourceError::Io {
                os_code: Some(13),
            }))),
            set_res: Ok(()),
        };
        let guard = init_with_environment(config, sink, &env);
        assert_eq!(guard.active_mode, TracingMode::ConsoleOnly);
        let diags = emitted.lock().unwrap();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].kind, DiagnosticKind::FileAppenderUnavailable);
        assert_eq!(diags[0].os_code, Some(13));
    }

    #[test]
    fn test_init_already_installed() {
        let emitted = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::new(MockSink {
            emitted: emitted.clone(),
        });
        let settings = ObservabilitySettings::for_build(true);
        let config = ObservabilityConfig {
            settings: ValidatedObservabilitySettings::try_from(settings).unwrap(),
            log_dir: LogDestination::Disabled,
        };
        let env = MockEnv {
            has_sub: true,
            dir_res: Ok(()),
            appender_res: Mutex::new(None),
            set_res: Ok(()),
        };
        let guard = init_with_environment(config, sink, &env);
        assert_eq!(guard.active_mode, TracingMode::ExistingSubscriber);
        let diags = emitted.lock().unwrap();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].kind, DiagnosticKind::SubscriberAlreadyInstalled);
        assert_eq!(
            diags[0].fallback,
            Some(DiagnosticFallback::ExistingSubscriber)
        );
    }

    #[test]
    fn test_subscriber_race() {
        let emitted = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::new(MockSink {
            emitted: emitted.clone(),
        });
        let settings = ObservabilitySettings::for_build(true);
        let config = ObservabilityConfig {
            settings: ValidatedObservabilitySettings::try_from(settings).unwrap(),
            log_dir: LogDestination::Disabled,
        };
        let env = MockEnv {
            has_sub: false,
            dir_res: Ok(()),
            appender_res: Mutex::new(None),
            set_res: Err(SubscriberInstallError::AlreadyInstalled),
        };
        let guard = init_with_environment(config, sink, &env);
        assert_eq!(guard.active_mode, TracingMode::ExistingSubscriber);
        let diags = emitted.lock().unwrap();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].kind, DiagnosticKind::SubscriberAlreadyInstalled);
    }

    struct BlockedWriter {
        unblocked: Arc<std::sync::atomic::AtomicBool>,
    }

    impl std::io::Write for BlockedWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            while !self.unblocked.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_guard_drop_flush_and_overflow() {
        let unblocked = Arc::new(AtomicBool::new(false));
        let writer = BlockedWriter {
            unblocked: unblocked.clone(),
        };

        let (non_blocking, worker_guard) =
            tracing_appender::non_blocking::NonBlockingBuilder::default()
                .lossy(true)
                .buffered_lines_limit(1)
                .finish(writer);

        let counter = non_blocking.error_counter().clone();

        let emitted = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::new(MockSink {
            emitted: emitted.clone(),
        });

        // Write rapidly to cause buffer overflow
        let mut writer_nb = non_blocking;
        for _ in 0..100 {
            let _ = writer_nb.write_all(b"test log message\n");
        }

        assert!(counter.dropped_lines() > 0);

        let guard = TracingGuard {
            worker_guard: Some(worker_guard),
            active_mode: TracingMode::FileAndConsole,
            error_counter: Some(counter),
            sink,
        };

        // Unblock blocked writer before dropping the guard so the thread finishes
        unblocked.store(true, Ordering::SeqCst);

        // Consuming shutdown
        let outcome = guard.shutdown(std::time::Duration::from_millis(500));
        assert_eq!(outcome, TracingShutdownOutcome::Flushed);
    }
}
