use super::*;
use crate::observability::config::ObservabilitySettings;
use crate::observability::error::ObservabilityValidationError;
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
}

#[tokio::test]
async fn test_shutdown_handles_graceful() {
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

    let report = shutdown_handles(Some(outbox_handle), Some(bridge_handle), None).await;
    assert_eq!(report.outbox_outcome, WorkerOutcome::Graceful);
    assert_eq!(report.bridge_outcome, WorkerOutcome::Graceful);
    assert_eq!(report.tracing_outcome, TracingShutdownOutcome::Flushed);
    assert!(report.is_graceful());
}

#[tokio::test]
async fn test_shutdown_handles_signal_failure() {
    // Construct outbox handle with closed channel to simulate signal failure
    let (outbox_tx, _) = mpsc::channel(1); // dropped rx
    let outbox_task = tokio::spawn(async move {});
    let outbox_handle = crate::bootstrap::workers::OutboxWorkerHandle {
        worker_task: Some(outbox_task),
        shutdown_tx: Some(outbox_tx),
    };

    let report = shutdown_handles(Some(outbox_handle), None, None).await;
    assert_eq!(report.outbox_outcome, WorkerOutcome::SignalFailed);
    assert_eq!(report.bridge_outcome, WorkerOutcome::AlreadyStopped);
}
