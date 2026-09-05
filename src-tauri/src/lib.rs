pub mod bootstrap;
pub mod commands;
pub mod dto;
pub mod observability;
mod runtime_shutdown;
pub mod state;

pub(crate) use runtime_shutdown::{RuntimeLifecycleAction, classify_run_event};
pub use runtime_shutdown::{
    RuntimeShutdownReport, TRACING_FLUSH_TIMEOUT, TracingShutdown, TracingShutdownOutcome,
    WorkerOutcome, WorkerShutdownReport, finalize_runtime_shutdown, shutdown_runtime,
};

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

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init());

    let builder = configure_updater(builder);

    let app = builder
        .setup(move |app| {
            bootstrap::setup(app, outbox_config, validated_settings)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::youtube_import::list_pending_youtube_imports_cmd,
            commands::youtube_import::resume_youtube_import_cmd,
            commands::youtube_import::discard_youtube_import_cmd,
            commands::project::create_project_cmd,
            commands::project_avatar::get_project_avatar_cmd,
            commands::project_avatar::set_project_avatar_cmd,
            commands::project::rename_project_cmd,
            commands::project::open_project_folder_cmd,
            commands::project::create_project_from_youtube_cmd,
            commands::project::list_youtube_subtitle_tracks_cmd,
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

fn configure_updater(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    let Some(public_key) = option_env!("AURALIS_UPDATER_PUBLIC_KEY") else {
        return builder;
    };
    let updater = tauri_plugin_updater::Builder::new()
        .pubkey(public_key)
        .build();
    builder.plugin(updater)
}

#[cfg(test)]
mod tests;
