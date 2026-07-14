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

    // 1. Setup storage and workers
    let (services, outbox_repo_opt) = storage::setup_storage(app)?;

    // 2. Setup Event Bridge and Coordinator
    let publisher = TauriEventPublisher::new(app_handle.clone());
    let coordinator = Arc::new(JobLifecycleCoordinator::new(
        services.project_repo.clone(),
        publisher.clone(),
    ));

    let mut event_bridge = TauriJobEventBridge::new(publisher, coordinator);

    if let Some(outbox_repo) = outbox_repo_opt {
        let outbox_shutdown = workers::spawn_outbox_worker(
            outbox_repo,
            services.artifact_store.clone(),
            services.artifact_index.clone(),
        );
        app.manage(outbox_shutdown);
    }

    // 3. Build Job Scheduler
    let job_manager =
        services::build_job_scheduler(services.job_repo.clone(), event_bridge.emitter());

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
    );

    Ok(())
}
