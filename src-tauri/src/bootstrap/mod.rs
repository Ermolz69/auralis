pub mod media_tools;
pub mod paths;
pub mod services;
pub mod storage;
pub mod usecases;
pub mod workers;

use adapters_tauri::{PreparedJobEventBridge, TauriEventPublisher};
use application::services::job_lifecycle_coordinator::JobLifecycleCoordinator;
use ports::error::PortError;
use std::sync::Arc;
use tauri::{App, Manager};

pub fn setup(
    app: &mut App,
    outbox_config: application::worker::outbox::maintenance::OutboxMaintenanceConfig,
    validated_settings: crate::observability::config::ValidatedObservabilitySettings,
) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle().clone();
    let app_paths = paths::AppPaths::resolve(app)?;
    app.manage(app_paths.clone());

    // 0a. Initialize observability
    let log_dir = crate::observability::config::LogDestination::Directory(app_paths.logs());

    let config = crate::observability::config::ObservabilityConfig {
        settings: validated_settings,
        log_dir,
    };

    let sink = Arc::new(crate::observability::diagnostic::StderrDiagnosticSink);
    let guard = crate::observability::init(config, sink);
    let mode_str = format!("{:?}", guard.active_mode);
    app.manage(crate::state::ManagedTracingGuard(std::sync::Mutex::new(
        Some(guard),
    )));

    tracing::info!(action = "observability_init", status = %mode_str, "Observability initialized");

    // 0. Compute workspace root
    let workspace_root = app_paths.workspaces();
    std::fs::create_dir_all(&workspace_root)?;

    // 1. Setup storage Adapter (fallible)
    let (services, outbox_repo) = storage::setup_storage(&app_paths)?;

    let temp_workspace = Arc::new(adapters_storage::local::LocalTempWorkspace::new(
        workspace_root.clone(),
    ));

    // 2. Prepare Event Bridge (does not spawn tasks yet)
    let prepared_bridge =
        PreparedJobEventBridge::new(adapters_tauri::JobEventBridgeConfig::default());

    // 3. Build Job Scheduler & load snapshots (fallible, before spawning anything)
    let job_manager = services::build_job_scheduler(
        services.job_repo.clone(),
        services.storage_uow.clone(),
        prepared_bridge.emitter(),
    )?;

    // 4. Register all static Tauri state & use cases BEFORE spawning any tasks
    app.manage(crate::state::ManagedJobRuntime(
        job_manager.clone() as Arc<dyn ports::job_runtime_control::JobRuntimeControlPort>
    ));

    usecases::setup_usecases(
        app.handle(),
        app_paths.projects(),
        services.project_repo.clone(),
        services.project_avatar_repo.clone(),
        services.artifact_index.clone(),
        services.artifact_store.clone(),
        services.storage_uow.clone(),
        job_manager.clone() as Arc<dyn ports::job_scheduler::JobSchedulerPort>,
        temp_workspace.clone(),
        job_manager.clone() as Arc<dyn ports::job_runtime_control::JobRuntimeControlPort>,
        services.youtube_imports.clone(),
    );
    app.manage(services.job_query.clone());

    // 5. Spawn background workers only after all fallible operations have succeeded
    let publisher = TauriEventPublisher::new(app_handle.clone());
    let coordinator = Arc::new(JobLifecycleCoordinator::new());

    let mut running_bridge = prepared_bridge.start(publisher.clone(), coordinator.clone());

    let outbox_shutdown = workers::spawn_outbox_worker(
        outbox_repo.clone(),
        &services,
        Arc::new(publisher.clone()),
        temp_workspace,
        outbox_config,
    );
    app.manage(crate::state::ManagedOutboxWorker(std::sync::Mutex::new(
        Some(outbox_shutdown),
    )));

    let bridge_handle = running_bridge
        .take_handle()
        .ok_or_else(|| PortError::Unexpected {
            message: "Event bridge handle unavailable".to_string(),
        })?;
    app.manage(crate::state::ManagedJobEventBridge(std::sync::Mutex::new(
        Some(bridge_handle),
    )));

    Ok(())
}
