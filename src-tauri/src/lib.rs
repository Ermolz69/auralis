pub mod bootstrap;
pub mod commands;
pub mod dto;
pub mod observability;
pub mod state;

#[derive(Debug)]
pub enum StartupError {
    Configuration(application::error::ApplicationError),
    TauriBuild(tauri::Error),
}

impl std::fmt::Display for StartupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StartupError::Configuration(e) => write!(f, "Configuration error: {}", e),
            StartupError::TauriBuild(e) => write!(f, "Tauri build error: {}", e),
        }
    }
}

impl std::error::Error for StartupError {}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), StartupError> {
    let outbox_config =
        application::worker::outbox::maintenance::OutboxMaintenanceConfig::try_default()
            .map_err(StartupError::Configuration)?;
    if let Err(e) = outbox_config.validate() {
        return Err(StartupError::Configuration(e));
    }

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            bootstrap::setup(app, outbox_config)?;
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
        .map_err(StartupError::TauriBuild)?;

    app.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            tauri::async_runtime::block_on(shutdown_runtime(app_handle));
        }
    });

    Ok(())
}

#[derive(Debug)]
pub enum WorkerOutcome {
    Graceful,
    Aborted,
    JoinFailed,
    AlreadyStopped,
    SignalFailed,
}

#[derive(Debug)]
pub struct RuntimeShutdownReport {
    pub outbox_outcome: WorkerOutcome,
    pub bridge_outcome: WorkerOutcome,
}

async fn shutdown_runtime(app_handle: &tauri::AppHandle) -> RuntimeShutdownReport {
    use crate::state::{ManagedJobEventBridge, ManagedOutboxWorker, ManagedTracingGuard};
    use tauri::Manager;

    tracing::info!("shutdown_runtime: initiating bounded shutdown");

    let outbox_handle_opt = app_handle
        .try_state::<ManagedOutboxWorker>()
        .and_then(|state| state.take());

    let bridge_handle_opt = app_handle
        .try_state::<ManagedJobEventBridge>()
        .and_then(|state| state.take());

    let mut outbox_outcome = WorkerOutcome::AlreadyStopped;
    let mut bridge_outcome = WorkerOutcome::AlreadyStopped;

    let mut outbox_task = None;
    if let Some(handle) = outbox_handle_opt {
        let (tx, task) = handle.into_shutdown_parts();
        if let Some(tx) = tx {
            if tx.send(()).await.is_err() {
                outbox_outcome = WorkerOutcome::SignalFailed;
            }
        }
        outbox_task = task;
    }

    let mut bridge_task = None;
    if let Some(handle) = bridge_handle_opt {
        let (tx, task) = handle.into_shutdown_parts();
        if let Some(tx) = tx {
            if tx.send(()).is_err() {
                bridge_outcome = WorkerOutcome::SignalFailed;
            }
        }
        bridge_task = task;
    }

    let deadline = tokio::time::sleep(std::time::Duration::from_secs(5));
    tokio::pin!(deadline);

    loop {
        if outbox_task.is_none() && bridge_task.is_none() {
            break; // all done
        }

        tokio::select! {
            _ = &mut deadline => {
                tracing::warn!("shutdown_runtime: global deadline reached, aborting remaining tasks");
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
            }, if outbox_task.is_some() => {
                outbox_task = None;
                if outbox_outcome.is_graceful_eligible() {
                    match res {
                        Ok(_) => outbox_outcome = WorkerOutcome::Graceful,
                        Err(e) if e.is_cancelled() => outbox_outcome = WorkerOutcome::Aborted,
                        Err(_) => outbox_outcome = WorkerOutcome::JoinFailed,
                    }
                }
            }
            res = async {
                match &mut bridge_task {
                    Some(t) => t.await,
                    None => std::future::pending().await,
                }
            }, if bridge_task.is_some() => {
                bridge_task = None;
                if bridge_outcome.is_graceful_eligible() {
                    match res {
                        Ok(_) => bridge_outcome = WorkerOutcome::Graceful,
                        Err(e) if e.is_cancelled() => bridge_outcome = WorkerOutcome::Aborted,
                        Err(_) => bridge_outcome = WorkerOutcome::JoinFailed,
                    }
                }
            }
        }
    }

    // Await aborted tasks if any
    if let Some(task) = outbox_task {
        match task.await {
            Err(e) if e.is_cancelled() => outbox_outcome = WorkerOutcome::Aborted,
            _ => outbox_outcome = WorkerOutcome::JoinFailed,
        }
    }
    if let Some(task) = bridge_task {
        match task.await {
            Err(e) if e.is_cancelled() => bridge_outcome = WorkerOutcome::Aborted,
            _ => bridge_outcome = WorkerOutcome::JoinFailed,
        }
    }

    let report = RuntimeShutdownReport {
        outbox_outcome,
        bridge_outcome,
    };

    tracing::info!(report = ?report, "shutdown_runtime: bounded shutdown completed");

    if let Some(state) = app_handle.try_state::<ManagedTracingGuard>() {
        let _ = state.take();
    }

    report
}

impl WorkerOutcome {
    fn is_graceful_eligible(&self) -> bool {
        matches!(self, WorkerOutcome::AlreadyStopped)
    }
}
