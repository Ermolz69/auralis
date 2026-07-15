pub mod media_tools;
pub mod services;
pub mod storage;
pub mod usecases;
pub mod workers;

use adapters_tauri::{TauriEventPublisher, TauriJobEventBridge};
use application::services::job_lifecycle_coordinator::JobLifecycleCoordinator;
use std::sync::Arc;
use tauri::{App, Manager};

pub fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle().clone();

    // 0. Compute workspace root
    let app_path = app.path();
    let workspace_root = app_path
        .app_cache_dir()
        .unwrap_or_else(|_| app_path.app_data_dir().unwrap())
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
            outbox_repo,
            services.artifact_store.clone(),
            services.artifact_index.clone(),
            services.storage_uow.clone(),
            Arc::new(publisher.clone()),
            temp_workspace.clone(),
        );
        app.manage(outbox_shutdown);
    }

    // 3. Build Job Scheduler
    let job_manager = services::build_job_scheduler(
        services.job_repo.clone(),
        services.storage_uow.clone(),
        event_bridge.emitter(),
    );

    // 4. Register State
    app.manage(event_bridge.take_handle().unwrap());

    // 5. Build and register AppUseCases
    usecases::setup_usecases(
        app.handle(),
        services.project_repo,
        services.artifact_index,
        services.artifact_store,
        services.storage_uow,
        job_manager,
        temp_workspace,
    );

    Ok(())
}
