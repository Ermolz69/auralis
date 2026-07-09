pub mod events;
pub mod services;
pub mod storage;
pub mod workers;

use tauri::{App, Manager};

pub fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle().clone();

    // 1. Build event emitter
    let emitter = events::build_job_event_emitter(app_handle);

    // 2. Setup storage and workers
    let (services, outbox_repo_opt) = storage::setup_storage(app)?;

    if let Some(outbox_repo) = outbox_repo_opt {
        workers::spawn_outbox_worker(
            outbox_repo,
            services.artifact_store.clone(),
            services.artifact_index.clone(),
        );
    }

    // 3. Build Job Scheduler
    let job_manager = services::build_job_scheduler(services.job_repo.clone(), emitter);

    // 4. Register State
    app.manage(job_manager);
    app.manage(services.project_repo);
    app.manage(services.artifact_index);
    app.manage(services.artifact_store);

    Ok(())
}
