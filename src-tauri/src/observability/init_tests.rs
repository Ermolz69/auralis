#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::config::{LogDestination, ObservabilityConfig};
use super::config::{ObservabilitySettings, ValidatedObservabilitySettings};
use super::diagnostic::{DiagnosticFallback, DiagnosticKind, DiagnosticSink, ProcessDiagnostic};
use super::init::*;
use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing_subscriber::EnvFilter;

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
    appender_res: Mutex<Option<Result<super::sink::OwnedSink, ObservabilityResourceError>>>,
    set_res: Result<(), SubscriberInstallError>,
}

impl ObservabilityEnvironment for MockEnv {
    fn has_subscriber(&self) -> bool {
        self.has_sub
    }
    fn create_directory(&self, _path: &std::path::Path) -> Result<(), ObservabilityResourceError> {
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
    ) -> Result<super::sink::OwnedSink, ObservabilityResourceError> {
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
    let guard = init_with_environment(config, Some(sink), &env);
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
    let guard = init_with_environment(config, Some(sink), &env);
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
    let guard = init_with_environment(config, Some(sink), &env);
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
    let guard = init_with_environment(config, Some(sink), &env);
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
    let guard = init_with_environment(config, Some(sink), &env);
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

    let owned = super::sink::OwnedSink::new(writer, 1, true);
    let non_blocking = owned.writer.clone();

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
        file: Some(owned),
        console: None,
        sampler: None,
        active_mode: TracingMode::FileAndConsole,
        diagnostic: sink,
    };

    // Unblock blocked writer before dropping the guard so the thread finishes
    unblocked.store(true, Ordering::SeqCst);

    // Consuming shutdown
    let outcome = guard.shutdown(std::time::Duration::from_millis(500));
    assert_eq!(outcome.file, crate::TracingShutdownOutcome::Flushed);
}
