#![allow(clippy::unwrap_used)]
use super::{
    config::{
        LogDestination, ObservabilityConfig, ObservabilitySettings, ValidatedObservabilitySettings,
    },
    diagnostic::{DiagnosticKind, DiagnosticSink, ProcessDiagnostic},
    guard::TracingGuard,
    init::{
        ObservabilityEnvironment, ObservabilityResourceError, SubscriberInstallError,
        init_with_environment,
    },
    sink::OwnedSink,
};
use crate::TracingShutdownOutcome;
use std::{
    io::Write,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Clone)]
struct ControlledWriter {
    output: Arc<Mutex<Vec<u8>>>,
    gate: Arc<(Mutex<bool>, Condvar)>,
    entered: Arc<AtomicBool>,
}
impl Write for ControlledWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.entered.store(true, Ordering::Release);
        let (lock, wake) = &*self.gate;
        let mut open = lock.lock().unwrap();
        while !*open {
            open = wake.wait(open).unwrap();
        }
        self.output.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
struct Environment {
    console: Mutex<Option<OwnedSink>>,
    dispatch: Mutex<Option<tracing::Dispatch>>,
}
impl ObservabilityEnvironment for Environment {
    fn has_subscriber(&self) -> bool {
        false
    }
    fn create_directory(&self, _: &std::path::Path) -> Result<(), ObservabilityResourceError> {
        unreachable!()
    }
    fn build_non_blocking(
        &self,
        _: &std::path::Path,
        _: tracing_appender::rolling::Rotation,
        _: usize,
        _: usize,
        _: bool,
    ) -> Result<OwnedSink, ObservabilityResourceError> {
        unreachable!()
    }
    fn build_console(&self) -> OwnedSink {
        self.console.lock().unwrap().take().unwrap()
    }
    fn set_global_default(
        &self,
        dispatch: tracing::Dispatch,
    ) -> Result<(), SubscriberInstallError> {
        *self.dispatch.lock().unwrap() = Some(dispatch);
        Ok(())
    }
}

#[test]
fn console_shutdown_child() {
    if std::env::var("AURALIS_CONSOLE_SHUTDOWN_TEST").as_deref() != Ok("1") {
        return;
    }
    for blocked in [false, true] {
        let writer = ControlledWriter {
            output: Arc::default(),
            gate: Arc::new((Mutex::new(!blocked), Condvar::new())),
            entered: Arc::new(AtomicBool::new(false)),
        };
        let owner = OwnedSink::new(writer.clone(), 4, true);
        let counter = owner.writer.error_counter();
        let env = Environment {
            console: Mutex::new(Some(owner)),
            dispatch: Mutex::new(None),
        };
        let guard = init_with_environment(
            ObservabilityConfig {
                settings: ValidatedObservabilitySettings::try_from(
                    ObservabilitySettings::for_build(false),
                )
                .unwrap(),
                log_dir: LogDestination::Disabled,
            },
            None,
            &env,
        );
        assert!(
            guard.sampler.is_some(),
            "console-only must own a health sampler"
        );
        let dispatch = env.dispatch.lock().unwrap().take().unwrap();
        tracing::dispatcher::with_default(&dispatch, || tracing::info!(event_name = "tail_record"));
        let entered_deadline = Instant::now() + Duration::from_secs(2);
        while !writer.entered.load(Ordering::Acquire) {
            assert!(Instant::now() < entered_deadline);
            std::thread::yield_now();
        }
        if blocked {
            tracing::dispatcher::with_default(&dispatch, || {
                for _ in 0..100 {
                    tracing::info!(event_name = "queue_pressure");
                }
            });
            assert!(counter.dropped_lines() > 0);
        }
        let started = Instant::now();
        let report = guard.shutdown(Duration::from_millis(100));
        assert!(started.elapsed() < Duration::from_millis(500));
        assert_eq!(report.sampler, TracingShutdownOutcome::Flushed);
        assert_eq!(report.file, TracingShutdownOutcome::NotOwned);
        assert_eq!(
            report.console,
            if blocked {
                TracingShutdownOutcome::TimedOut
            } else {
                TracingShutdownOutcome::Flushed
            }
        );
        *writer.gate.0.lock().unwrap() = true;
        writer.gate.1.notify_all();
        if !blocked {
            assert!(
                String::from_utf8(writer.output.lock().unwrap().clone())
                    .unwrap()
                    .contains("tail_record")
            );
        }
        drop(dispatch);
    }
}

#[test]
fn console_shutdown_is_bounded_and_reports_each_owned_resource() {
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "observability::sink_tests::console_shutdown_child",
            "--nocapture",
        ])
        .env("AURALIS_CONSOLE_SHUTDOWN_TEST", "1")
        .env("RUST_LOG", "info")
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        if Instant::now() > deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("console shutdown child exceeded deadline");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn failed_sink_is_not_reported_as_flushed() {
    struct FailedWriter;
    impl Write for FailedWriter {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("SYNTHETIC_PRIVATE_ERROR"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let failures_before = super::bounded_writer::WRITE_FAILURES.load(Ordering::Relaxed);
    let owner = OwnedSink::new(FailedWriter, 4, true);
    owner.writer.clone().write_all(b"event").unwrap();
    assert_eq!(
        owner.shutdown(Duration::from_secs(1)),
        TracingShutdownOutcome::Failed
    );
    assert!(super::bounded_writer::WRITE_FAILURES.load(Ordering::Relaxed) > failures_before);
}

#[test]
fn zero_progress_sink_is_counted_and_not_reported_as_flushed() {
    struct ZeroWriter;
    impl Write for ZeroWriter {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Ok(0)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let failures_before = super::bounded_writer::WRITE_FAILURES.load(Ordering::Relaxed);
    let owner = OwnedSink::new(ZeroWriter, 4, true);
    owner.writer.clone().write_all(b"event").unwrap();
    assert_eq!(
        owner.shutdown(Duration::from_secs(1)),
        TracingShutdownOutcome::Failed
    );
    assert!(super::bounded_writer::WRITE_FAILURES.load(Ordering::Relaxed) > failures_before);
}

#[derive(Default)]
struct CapturingDiagnosticSink(Mutex<Vec<ProcessDiagnostic>>);

impl DiagnosticSink for CapturingDiagnosticSink {
    fn emit(&self, diagnostic: ProcessDiagnostic) {
        self.0.lock().unwrap().push(diagnostic);
    }
}

#[test]
fn failed_shutdown_has_failure_diagnostic() {
    struct FailedWriter;
    impl Write for FailedWriter {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("private failure"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let diagnostic = Arc::new(CapturingDiagnosticSink::default());
    let owner = OwnedSink::new(FailedWriter, 4, true);
    owner.writer.clone().write_all(b"event").unwrap();
    let report = TracingGuard {
        file: Some(owner),
        console: None,
        sampler: None,
        active_mode: super::init::TracingMode::ExistingSubscriber,
        diagnostic: diagnostic.clone(),
    }
    .shutdown(Duration::from_secs(1));

    assert_eq!(report.file, TracingShutdownOutcome::Failed);
    assert_eq!(diagnostic.0.lock().unwrap().len(), 1);
    assert_eq!(
        diagnostic.0.lock().unwrap()[0].kind,
        DiagnosticKind::TracingFlushFailed
    );
}

#[test]
fn timed_out_shutdown_has_timeout_diagnostic() {
    let writer = ControlledWriter {
        output: Arc::default(),
        gate: Arc::new((Mutex::new(false), Condvar::new())),
        entered: Arc::new(AtomicBool::new(false)),
    };
    let diagnostic = Arc::new(CapturingDiagnosticSink::default());
    let owner = OwnedSink::new(writer.clone(), 4, true);
    owner.writer.clone().write_all(b"event").unwrap();
    let entered_deadline = Instant::now() + Duration::from_secs(1);
    while !writer.entered.load(Ordering::Acquire) {
        assert!(Instant::now() < entered_deadline);
        std::thread::yield_now();
    }

    let report = TracingGuard {
        file: Some(owner),
        console: None,
        sampler: None,
        active_mode: super::init::TracingMode::ExistingSubscriber,
        diagnostic: diagnostic.clone(),
    }
    .shutdown(Duration::from_millis(20));

    assert_eq!(report.file, TracingShutdownOutcome::TimedOut);
    assert_eq!(diagnostic.0.lock().unwrap().len(), 1);
    assert_eq!(
        diagnostic.0.lock().unwrap()[0].kind,
        DiagnosticKind::TracingFlushTimedOut
    );
    *writer.gate.0.lock().unwrap() = true;
    writer.gate.1.notify_all();
}
