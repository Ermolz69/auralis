pub mod media_tools;
pub mod services;
pub mod storage;
pub mod usecases;
pub mod workers;

use adapters_tauri::{TauriEventPublisher, TauriJobEventBridge};
use application::services::job_lifecycle_coordinator::JobLifecycleCoordinator;
use std::sync::Arc;
use tauri::{App, Manager};

pub fn setup(
    app: &mut App,
    outbox_config: application::worker::outbox::maintenance::OutboxMaintenanceConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle().clone();

    // 0a. Initialize observability
    let log_dir = match app.path().app_log_dir() {
        Ok(dir) => crate::observability::config::LogDestination::Directory(dir),
        Err(_) => crate::observability::config::LogDestination::Unavailable(
            crate::observability::config::LogDestinationErrorKind::PathResolutionFailed,
        ),
    };

    let is_debug = cfg!(debug_assertions);
    let mut config =
        crate::observability::config::ObservabilityConfig::for_build(log_dir, is_debug);
    let sink = crate::observability::init::StderrDiagnosticSink;

    if let Err(e) = config.validate() {
        use crate::observability::init::DiagnosticSink;
        sink.emit_warning(&format!("Invalid observability config: {}", e));
        config.log_dir = crate::observability::config::LogDestination::Disabled;
    }

    let guard = crate::observability::init(config, &sink);
    let mode_str = format!("{:?}", guard.active_mode);
    app.manage(crate::state::ManagedTracingGuard(std::sync::Mutex::new(
        Some(guard),
    )));

    tracing::info!(action = "observability_init", status = %mode_str, "Observability initialized");

    // 0. Compute workspace root
    let app_path = app.path();
    let workspace_root = match app_path.app_cache_dir() {
        Ok(dir) => dir,
        Err(_) => app_path.app_data_dir()?,
    }
    .join("workspaces");
    std::fs::create_dir_all(&workspace_root)?;

    // 1. Setup storage and workers
    let (services, outbox_repo_opt) = storage::setup_storage(app, &workspace_root)?;

    let temp_workspace = Arc::new(adapters_storage::local::LocalTempWorkspace::new(
        workspace_root.clone(),
    ));

    // 2. Setup Event Bridge and Coordinator
    let publisher = TauriEventPublisher::new(app_handle.clone());
    let coordinator = Arc::new(JobLifecycleCoordinator::new());

    let mut event_bridge = TauriJobEventBridge::new(publisher.clone(), coordinator);

    if let Some(outbox_repo) = outbox_repo_opt {
        let outbox_shutdown = workers::spawn_outbox_worker(
            outbox_repo.clone(),
            services.artifact_store.clone(),
            services.artifact_index.clone(),
            services.storage_uow.clone(),
            Arc::new(publisher.clone()),
            temp_workspace.clone(),
            outbox_config,
        );
        app.manage(crate::state::ManagedOutboxWorker(std::sync::Mutex::new(
            Some(outbox_shutdown),
        )));
    } else {
        app.manage(crate::state::ManagedOutboxWorker(std::sync::Mutex::new(
            None,
        )));
    }

    // 3. Build Job Scheduler
    let job_manager = services::build_job_scheduler(
        services.job_repo.clone(),
        services.storage_uow.clone(),
        event_bridge.emitter(),
    )?;

    // 4. Register State
    let bridge_handle = event_bridge
        .take_handle()
        .ok_or("Failed to spawn event bridge")?;
    app.manage(crate::state::ManagedJobEventBridge(std::sync::Mutex::new(
        Some(bridge_handle),
    )));

    // 5. Build and register AppUseCases
    usecases::setup_usecases(
        app.handle(),
        services.project_repo,
        services.artifact_index,
        services.artifact_store,
        services.storage_uow,
        job_manager.clone() as Arc<dyn ports::job_scheduler::JobSchedulerPort>,
        temp_workspace,
        job_manager.clone() as Arc<dyn ports::job_runtime_control::JobRuntimeControlPort>,
    );

    Ok(())
}
