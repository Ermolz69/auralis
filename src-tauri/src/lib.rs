pub mod bootstrap;
pub mod commands;
pub mod dto;
pub mod observability;
pub mod state;

use crate::observability::config::ValidatedObservabilitySettings;

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

pub fn prepare_runtime_config(
    settings: crate::observability::config::ObservabilitySettings,
) -> Result<ValidatedObservabilitySettings, AppRunError> {
    let validated = ValidatedObservabilitySettings::try_from(settings)?;
    Ok(validated)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), AppRunError> {
    let outbox_config =
        application::worker::outbox::maintenance::OutboxMaintenanceConfig::try_default()
            .map_err(AppRunError::Configuration)?;
    if let Err(e) = outbox_config.validate() {
        return Err(AppRunError::Configuration(e));
    }

    let is_debug = cfg!(debug_assertions);
    let settings = crate::observability::config::ObservabilitySettings::for_build(is_debug);
    let validated_settings = prepare_runtime_config(settings)?;

    let shutdown_report = std::sync::Arc::new(std::sync::Mutex::new(None));
    let shutdown_report_clone = shutdown_report.clone();

    let shutdown_timeout = outbox_config.shutdown_timeout;

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            bootstrap::setup(app, outbox_config, validated_settings)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::project::create_project_cmd,
            commands::project::create_project_from_youtube_cmd,
            commands::project::get_transcript_cmd,
            commands::project::get_project_cmd,
            commands::project::list_projects_cmd,
            commands::project::delete_project_cmd,
            commands::project::start_project_mock_pipeline_cmd,
            commands::artifact::list_project_artifacts_cmd,
            commands::artifact::resolve_artifact_path_cmd,
            commands::job::health_check,
            commands::job::list_jobs_cmd,
            commands::job::list_jobs_snapshot_cmd,
            commands::job::cancel_job_cmd,
            commands::media::probe_local_media_cmd,
            commands::media::import_local_media_cmd
        ])
        .build(tauri::generate_context!())
        .map_err(AppRunError::TauriBuild)?;

    app.run(move |app_handle, event| {
        if let RuntimeLifecycleAction::FinalShutdown = classify_run_event(&event) {
            use crate::state::{
                ManagedJobEventBridge, ManagedJobRuntime, ManagedOutboxWorker, ManagedTracingGuard,
            };
            use tauri::Manager;

            let job_runtime = app_handle
                .try_state::<ManagedJobRuntime>()
                .map(|state| state.0.clone());

            let jobs_report = if let Some(runtime) = job_runtime {
                match tauri::async_runtime::block_on(runtime.drain_all(shutdown_timeout)) {
                    Ok(rep) => rep,
                    Err(ports::error::PortError::AlreadyStopped) => {
                        ports::job_runtime_control::RuntimeShutdownReport::default()
                    }
                    Err(_e) => {
                        tracing::error!(
                            error = %common::observability::redaction::DiagnosticError {
                                kind: "JobRuntimeDrainFailed",
                                code: None,
                                retryable: false,
                            },
                            "job runtime drain failed"
                        );
                        ports::job_runtime_control::RuntimeShutdownReport::default()
                    }
                }
            } else {
                ports::job_runtime_control::RuntimeShutdownReport::default()
            };

            let outbox = app_handle
                .try_state::<ManagedOutboxWorker>()
                .and_then(|state| state.take());

            let bridge = app_handle
                .try_state::<ManagedJobEventBridge>()
                .and_then(|state| state.take());

            let tracing = app_handle
                .try_state::<ManagedTracingGuard>()
                .and_then(|state| state.take());

            let workers_report =
                tauri::async_runtime::block_on(shutdown_runtime(outbox, bridge, shutdown_timeout));
            let final_report = finalize_runtime_shutdown(workers_report, jobs_report, tracing);

            let mut guard = shutdown_report_clone
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            *guard = Some(final_report);
        }
    });

    let report_opt = shutdown_report
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take();
    match report_opt {
        Some(report) => {
            if report.is_graceful() {
                Ok(())
            } else {
                Err(AppRunError::Shutdown(report))
            }
        }
        None => Err(AppRunError::ShutdownNotObserved),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeLifecycleAction {
    Ignore,
    FinalShutdown,
}

pub(crate) fn classify_run_event(event: &tauri::RunEvent) -> RuntimeLifecycleAction {
    match event {
        tauri::RunEvent::Exit => RuntimeLifecycleAction::FinalShutdown,
        _ => RuntimeLifecycleAction::Ignore,
    }
}

pub trait TracingShutdown {
    fn shutdown(self, timeout: std::time::Duration) -> TracingShutdownOutcome;
}

impl TracingShutdown for crate::observability::init::TracingGuard {
    fn shutdown(self, timeout: std::time::Duration) -> TracingShutdownOutcome {
        crate::observability::init::TracingGuard::shutdown(self, timeout)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerOutcome {
    Graceful,
    Aborted,
    JoinFailed,
    AlreadyStopped,
    SignalFailed,
}

impl WorkerOutcome {
    pub fn is_graceful(&self) -> bool {
        matches!(
            self,
            WorkerOutcome::Graceful | WorkerOutcome::AlreadyStopped
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TracingShutdownOutcome {
    Flushed,
    TimedOut,
    NotOwned,
    FlushThreadStartFailed,
}

impl TracingShutdownOutcome {
    pub fn is_graceful(&self) -> bool {
        matches!(
            self,
            TracingShutdownOutcome::Flushed | TracingShutdownOutcome::NotOwned
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerShutdownReport {
    pub outbox_outcome: WorkerOutcome,
    pub bridge_outcome: WorkerOutcome,
}

impl WorkerShutdownReport {
    pub fn is_graceful(&self) -> bool {
        self.outbox_outcome.is_graceful() && self.bridge_outcome.is_graceful()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeShutdownReport {
    pub outbox_outcome: WorkerOutcome,
    pub bridge_outcome: WorkerOutcome,
    pub jobs_outcome: ports::job_runtime_control::RuntimeShutdownReport,
    pub tracing_outcome: TracingShutdownOutcome,
}

impl RuntimeShutdownReport {
    pub fn is_graceful(&self) -> bool {
        self.outbox_outcome.is_graceful()
            && self.bridge_outcome.is_graceful()
            && self.tracing_outcome.is_graceful()
            && self.jobs_outcome.forced_aborted_count == 0
            && self.jobs_outcome.unconfirmed_count == 0
            && self.jobs_outcome.join_failed_count == 0
    }
}

pub const TRACING_FLUSH_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(500);

pub fn finalize_runtime_shutdown<T: TracingShutdown>(
    workers: WorkerShutdownReport,
    jobs_outcome: ports::job_runtime_control::RuntimeShutdownReport,
    tracing: Option<T>,
) -> RuntimeShutdownReport {
    tracing::info!(
        action = "workers_shutdown_completed",
        outbox_outcome = ?workers.outbox_outcome,
        bridge_outcome = ?workers.bridge_outcome,
        jobs_outcome = ?jobs_outcome,
        "shutdown_handles: workers finished, initiating tracing flush"
    );

    let tracing_outcome = if let Some(guard) = tracing {
        guard.shutdown(TRACING_FLUSH_TIMEOUT)
    } else {
        TracingShutdownOutcome::NotOwned
    };

    RuntimeShutdownReport {
        outbox_outcome: workers.outbox_outcome,
        bridge_outcome: workers.bridge_outcome,
        jobs_outcome,
        tracing_outcome,
    }
}

pub async fn shutdown_runtime(
    outbox: Option<crate::bootstrap::workers::OutboxWorkerHandle>,
    bridge: Option<adapters_tauri::job_event_bridge::JobEventBridgeHandle>,
    timeout: std::time::Duration,
) -> WorkerShutdownReport {
    // 1. Start overall deadline before sending signals
    let deadline = tokio::time::sleep(timeout);
    tokio::pin!(deadline);

    let mut outbox_outcome = WorkerOutcome::AlreadyStopped;
    let mut bridge_outcome = WorkerOutcome::AlreadyStopped;

    let mut outbox_task = None;
    if let Some(handle) = outbox {
        let (tx, task) = handle.into_shutdown_parts();
        if let Some(tx) = tx {
            // Signal outbox via try_send
            // Note: Since this channel is used only for shutdown, Full is treated as a success
            match tx.try_send(()) {
                Ok(_) => {}
                Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {}
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                    outbox_outcome = WorkerOutcome::SignalFailed;
                }
            }
        }
        outbox_task = task;
    }

    let mut bridge_task = None;
    if let Some(handle) = bridge {
        let (tx, task) = handle.into_shutdown_parts();
        if let Some(tx) = tx {
            if tx.send(()).is_err() {
                bridge_outcome = WorkerOutcome::SignalFailed;
            }
        }
        bridge_task = task;
    }

    let mut outbox_done = outbox_task.is_none();
    let mut bridge_done = bridge_task.is_none();

    while !outbox_done || !bridge_done {
        tokio::select! {
            _ = &mut deadline => {
                tracing::warn!(
                    action = "shutdown_timeout",
                    "shutdown_runtime: global deadline reached, aborting remaining tasks"
                );
                if let Some(ref task) = outbox_task {
                    task.abort();
                }
                if let Some(ref task) = bridge_task {
                    task.abort();
                }
                break;
            }
            res = async {
                match &mut outbox_task {
                    Some(t) => t.await,
                    None => std::future::pending().await,
                }
            }, if !outbox_done => {
                outbox_done = true;
                update_outcome(&mut outbox_outcome, res);
                outbox_task = None;
            }
            res = async {
                match &mut bridge_task {
                    Some(t) => t.await,
                    None => std::future::pending().await,
                }
            }, if !bridge_done => {
                bridge_done = true;
                update_outcome(&mut bridge_outcome, res);
                bridge_task = None;
            }
        }
    }

    // Await aborted tasks if any
    if let Some(task) = outbox_task {
        let res = task.await;
        update_outcome(&mut outbox_outcome, res);
    }
    if let Some(task) = bridge_task {
        let res = task.await;
        update_outcome(&mut bridge_outcome, res);
    }

    WorkerShutdownReport {
        outbox_outcome,
        bridge_outcome,
    }
}

fn update_outcome(current: &mut WorkerOutcome, task_result: Result<(), tokio::task::JoinError>) {
    if matches!(current, WorkerOutcome::SignalFailed) {
        return;
    }
    match task_result {
        Ok(_) => *current = WorkerOutcome::Graceful,
        Err(e) if e.is_cancelled() => *current = WorkerOutcome::Aborted,
        Err(_) => *current = WorkerOutcome::JoinFailed,
    }
}

#[cfg(test)]
mod tests;
