#![allow(clippy::unwrap_used)]
use super::*;
use crate::observability::config::ObservabilitySettings;
use crate::observability::error::ObservabilityValidationError;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

#[test]
fn test_app_run_error_display_safety() {
    // 1. Verify Configuration error does not leak nested details in Display
    let inner_app_err = application::error::ApplicationError::Configuration(
        "C:/secret/path/to/database.db".to_string(),
    );
    let err = AppRunError::Configuration(inner_app_err);
    assert_eq!(err.to_string(), "application configuration is invalid");

    // 2. Verify Observability error format
    let inner_obs_err = ObservabilityValidationError::ZeroMaxLogFiles;
    let err2 = AppRunError::Observability(inner_obs_err);
    assert_eq!(err2.to_string(), "observability configuration is invalid");

    // 3. Verify TauriBuild error format
    let inner_tauri_err = tauri::Error::Setup(
        (Box::new(std::io::Error::other("dummy")) as Box<dyn std::error::Error>).into(),
    );
    let err3 = AppRunError::TauriBuild(inner_tauri_err);
    assert_eq!(err3.to_string(), "failed to build Tauri application");
}

#[test]
fn test_prepare_runtime_config_validation() {
    let mut settings = ObservabilitySettings::for_build(true);

    // Test valid default filter
    settings.default_filter = "info".to_string();
    assert!(prepare_runtime_config(settings.clone()).is_ok());

    // Test invalid default filter
    settings.default_filter = "!!!invalid!!!".to_string();
    let res = prepare_runtime_config(settings.clone());
    assert!(res.is_err());
    if let Err(AppRunError::Observability(e)) = res {
        assert_eq!(e, ObservabilityValidationError::InvalidDefaultFilter);
    } else {
        panic!("Expected ObservabilityValidationError::InvalidDefaultFilter");
    }
}

#[test]
fn test_runtime_shutdown_report_graceful_logic() {
    let mut report = RuntimeShutdownReport {
        outbox_outcome: WorkerOutcome::Graceful,
        bridge_outcome: WorkerOutcome::AlreadyStopped,
        jobs_outcome: ports::job_runtime_control::RuntimeShutdownReport::default(),
        tracing_outcome: TracingShutdownOutcome::Flushed,
    };
    assert!(report.is_graceful());

    report.tracing_outcome = TracingShutdownOutcome::NotOwned;
    assert!(report.is_graceful());

    report.outbox_outcome = WorkerOutcome::SignalFailed;
    assert!(!report.is_graceful());

    report.outbox_outcome = WorkerOutcome::Graceful;
    report.tracing_outcome = TracingShutdownOutcome::TimedOut;
    assert!(!report.is_graceful());

    // Timeout/abort in jobs makes report non-graceful
    report.tracing_outcome = TracingShutdownOutcome::Flushed;
    report.jobs_outcome.forced_aborted_count = 1;
    assert!(!report.is_graceful());
}

#[tokio::test]
async fn test_shutdown_runtime_graceful() {
    // Spawn graceful outbox worker
    let (outbox_tx, mut outbox_rx) = mpsc::channel(1);
    let outbox_task = tokio::spawn(async move {
        let _ = outbox_rx.recv().await;
    });
    let outbox_handle = crate::bootstrap::workers::OutboxWorkerHandle {
        worker_task: Some(outbox_task),
        shutdown_tx: Some(outbox_tx),
    };

    // Spawn graceful bridge worker
    let (bridge_tx, bridge_rx) = oneshot::channel();
    let bridge_task = tokio::spawn(async move {
        let _ = bridge_rx.await;
    });
    let bridge_handle = adapters_tauri::job_event_bridge::JobEventBridgeHandle {
        shutdown_tx: Some(bridge_tx),
        worker_handle: Some(bridge_task),
    };

    let report = shutdown_runtime(
        Some(outbox_handle),
        Some(bridge_handle),
        Duration::from_secs(5),
    )
    .await;
    assert_eq!(report.outbox_outcome, WorkerOutcome::Graceful);
    assert_eq!(report.bridge_outcome, WorkerOutcome::Graceful);
}

#[tokio::test]
async fn test_shutdown_runtime_signal_failure() {
    // Construct outbox handle with closed channel to simulate signal failure
    let (outbox_tx, _) = mpsc::channel(1); // dropped rx
    let outbox_task = tokio::spawn(async move {});
    let outbox_handle = crate::bootstrap::workers::OutboxWorkerHandle {
        worker_task: Some(outbox_task),
        shutdown_tx: Some(outbox_tx),
    };

    let report = shutdown_runtime(Some(outbox_handle), None, Duration::from_secs(5)).await;
    assert_eq!(report.outbox_outcome, WorkerOutcome::SignalFailed);
    assert_eq!(report.bridge_outcome, WorkerOutcome::AlreadyStopped);
}

#[tokio::test]
async fn test_shutdown_runtime_timeout_abort() {
    // Outbox task ignores signal and hangs
    let (outbox_tx, mut outbox_rx) = mpsc::channel(1);
    let outbox_task = tokio::spawn(async move {
        let _ = outbox_rx.recv().await;
        std::future::pending::<()>().await;
    });
    let outbox_handle = crate::bootstrap::workers::OutboxWorkerHandle {
        worker_task: Some(outbox_task),
        shutdown_tx: Some(outbox_tx),
    };

    // Call with short timeout
    let report = shutdown_runtime(Some(outbox_handle), None, Duration::from_millis(50)).await;
    assert_eq!(report.outbox_outcome, WorkerOutcome::Aborted);
    assert_eq!(report.bridge_outcome, WorkerOutcome::AlreadyStopped);
}

#[test]
fn test_classify_run_event() {
    assert_eq!(
        classify_run_event(&tauri::RunEvent::Exit),
        RuntimeLifecycleAction::FinalShutdown
    );
    assert_eq!(
        classify_run_event(&tauri::RunEvent::Ready),
        RuntimeLifecycleAction::Ignore
    );
}

struct MockTracingShutdown {
    called: Arc<std::sync::atomic::AtomicBool>,
}

impl TracingShutdown for MockTracingShutdown {
    fn shutdown(self, _timeout: std::time::Duration) -> TracingShutdownOutcome {
        self.called.store(true, std::sync::atomic::Ordering::SeqCst);
        TracingShutdownOutcome::Flushed
    }
}

#[test]
fn test_finalize_runtime_shutdown_tracing() {
    let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let workers = WorkerShutdownReport {
        outbox_outcome: WorkerOutcome::Graceful,
        bridge_outcome: WorkerOutcome::Graceful,
    };

    let report = finalize_runtime_shutdown(
        workers,
        ports::job_runtime_control::RuntimeShutdownReport::default(),
        Some(MockTracingShutdown {
            called: called.clone(),
        }),
    );

    assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(report.outbox_outcome, WorkerOutcome::Graceful);
    assert_eq!(report.bridge_outcome, WorkerOutcome::Graceful);
    assert_eq!(report.tracing_outcome, TracingShutdownOutcome::Flushed);
}

#[test]
fn test_shutdown_logging_redaction() {
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone)]
    struct MockWriter {
        buf: Arc<Mutex<Vec<u8>>>,
    }

    impl<'a> MakeWriter<'a> for MockWriter {
        type Writer = Self;
        fn make_writer(&self) -> Self::Writer {
            self.clone()
        }
    }

    impl std::io::Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.buf.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let buf = Arc::new(Mutex::new(Vec::new()));
    let writer = MockWriter { buf: buf.clone() };
    let subscriber = tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        tracing::error!(
            error = %common::observability::redaction::DiagnosticError {
                kind: "JobRuntimeDrainFailed",
                code: None,
                retryable: false,
            },
            "job runtime drain failed"
        );
    });

    let logs = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
    assert!(logs.contains("JobRuntimeDrainFailed"));
    assert!(logs.contains("job runtime drain failed"));

    assert!(!logs.contains("secret"));
    assert!(!logs.contains("SECRET"));
    assert!(!logs.contains("token"));
    assert!(!logs.contains("Bearer"));
    assert!(!logs.contains("sqlx"));
}
