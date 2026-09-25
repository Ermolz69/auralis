mod app_root_lease;
pub mod media_tools;
#[cfg(feature = "native-e2e")]
pub mod native_e2e_subtitle_source;
pub mod paths;
pub mod selected_translation_runtime;
pub mod services;
pub mod storage;
pub mod translation;
mod translation_runtime;
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
    record_native_e2e_checkpoint("setup-started");
    let app_handle = app.handle().clone();
    let app_paths = paths::AppPaths::resolve(app)?;
    let storage_lease = Arc::new(app_root_lease::AppRootLease::acquire(app_paths.root())?);
    app.manage(storage_lease.clone());
    app.manage(app_paths.clone());
    record_native_e2e_checkpoint("paths-ready");

    // 0a. Initialize observability
    let log_dir = crate::observability::config::LogDestination::Directory(app_paths.logs());

    let config = crate::observability::config::ObservabilityConfig {
        settings: validated_settings,
        log_dir,
    };

    let guard = crate::observability::init(config);
    let mode_str = format!("{:?}", guard.active_mode);
    app.manage(crate::state::ManagedTracingGuard(std::sync::Mutex::new(
        Some(guard),
    )));
    record_native_e2e_checkpoint("observability-ready");

    tracing::info!(action = "observability_init", status = %mode_str, "Observability initialized");

    // 0. Compute workspace root
    let workspace_root = app_paths.workspaces();
    std::fs::create_dir_all(&workspace_root)?;

    // 1. Setup storage Adapter (fallible)
    let (services, outbox_repo) = storage::setup_storage(&app_paths, storage_lease)?;
    record_native_e2e_checkpoint("storage-ready");

    let translations = translation::TranslationUseCases::new(
        services.project_repo.clone(),
        services.artifact_index.clone(),
        services.artifact_store.clone(),
        services.storage_uow.clone(),
        services.translation_links.clone(),
        services.translation_host_jobs.clone(),
        app_paths.translate_database(),
    );
    let packages = Arc::new(adapters_translate::TranslationPackageService::new(
        app_paths.translation_packages(),
        app_paths.translation_download_cache(),
        app_paths.translation_package_selection(),
    ));
    let initial_runtime = match packages.selected_runtime() {
        Ok(Some(runtime)) => Some(runtime),
        Ok(None) => translation_runtime::load(&app_paths)?,
        Err(error) => {
            tracing::warn!(error = %error, "selected translation package needs attention");
            None
        }
    };
    app.manage(selected_translation_runtime::SelectedTranslationRuntime::new(initial_runtime));
    app.manage(packages);
    for item in tauri::async_runtime::block_on(translations.reconcile.execute())? {
        if let Err(error) = item.outcome {
            tracing::warn!(
                translation_id = %item.translation_id,
                error = %error,
                "translation run registration needs attention"
            );
        }
    }
    for item in tauri::async_runtime::block_on(translations.recover_jobs.execute())? {
        if let Err(error) = item.outcome {
            tracing::warn!(
                job_id = %item.job_id,
                run_id = %item.run_id,
                error = %error,
                "translation host job recovery needs attention"
            );
        }
    }
    for item in tauri::async_runtime::block_on(translations.recover_publications.execute())? {
        if let Err(error) = item.outcome {
            tracing::warn!(
                translation_id = %item.translation_id,
                error = %error,
                "translation result publication recovery needs attention"
            );
        }
    }
    app.manage(Arc::new(translations));
    record_native_e2e_checkpoint("translations-ready");

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
    record_native_e2e_checkpoint("scheduler-ready");

    // 4. Register all static Tauri state & use cases BEFORE spawning any tasks
    app.manage(crate::state::ManagedJobRuntime(
        job_manager.clone() as Arc<dyn ports::job_runtime_control::JobRuntimeControlPort>
    ));

    usecases::setup_usecases(
        app.handle(),
        usecases::AppUseCaseDependencies {
            ui_preferences: services.ui_preferences.clone(),
            artifact_recovery: Arc::new(outbox_repo.clone()),
            projects_root: app_paths.projects(),
            project_repo: services.project_repo.clone(),
            project_avatar_repo: services.project_avatar_repo.clone(),
            artifact_index: services.artifact_index.clone(),
            artifact_store: services.artifact_store.clone(),
            storage_uow: services.storage_uow.clone(),
            job_scheduler: job_manager.clone() as Arc<dyn ports::job_scheduler::JobSchedulerPort>,
            workspace_port: temp_workspace.clone(),
            job_runtime: job_manager.clone()
                as Arc<dyn ports::job_runtime_control::JobRuntimeControlPort>,
            youtube_imports: services.youtube_imports.clone(),
        },
    )?;
    app.manage(services.job_query.clone());
    record_native_e2e_checkpoint("use-cases-ready");

    // 5. Spawn background workers only after all fallible operations have succeeded
    let publisher = TauriEventPublisher::new(app_handle.clone());
    let coordinator = Arc::new(JobLifecycleCoordinator::new());

    let mut running_bridge = prepared_bridge.start(publisher.clone(), coordinator.clone());

    let outbox_shutdown = workers::spawn_outbox_worker(
        outbox_repo.clone(),
        &services,
        Arc::new(publisher.clone()),
        temp_workspace,
        app_paths.translate_database(),
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
    record_native_e2e_checkpoint("setup-complete");

    Ok(())
}

pub(crate) fn record_native_e2e_checkpoint(checkpoint: &str) {
    if !cfg!(feature = "native-e2e") {
        return;
    }
    let Some(root) = std::env::var_os("AURALIS_NATIVE_E2E_DATA_DIR") else {
        return;
    };
    let path = std::path::PathBuf::from(root).join("native-e2e-bootstrap.txt");
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = std::io::Write::write_fmt(&mut file, format_args!("{checkpoint}\n"));
    }
}
