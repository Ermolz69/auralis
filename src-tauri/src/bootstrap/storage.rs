use super::services::RuntimeServices;
use adapters_storage::local::artifact_store::LocalArtifactStore;
use adapters_storage::memory::{InMemoryArtifactIndex, InMemoryProjectRepository};
use adapters_storage::sqlite::{
    SqliteArtifactIndex, SqliteJobRepository, SqliteOutboxRepository, SqliteProjectRepository,
    SqliteStorageUnitOfWork,
};
use ports::repository::JobRepository;
use std::sync::Arc;
use tauri::{App, Manager};

pub fn setup_storage(
    app: &App,
) -> Result<(RuntimeServices, Option<SqliteOutboxRepository>), Box<dyn std::error::Error>> {
    if std::env::var("AURALIS_STORAGE").unwrap_or_default() == "in-memory" {
        println!("WARNING: Running with IN-MEMORY storage adapter! Data will be lost on exit.");
        let db = Arc::new(std::sync::Mutex::new(
            adapters_storage::memory::InMemoryDatabase::new(),
        ));
        let project_repo = Arc::new(InMemoryProjectRepository::new(db.clone()));
        let job_repo = Arc::new(adapters_storage::memory::InMemoryJobRepository::new(
            db.clone(),
        ));
        let artifact_index = Arc::new(InMemoryArtifactIndex::new());
        let artifact_store = Arc::new(LocalArtifactStore::new(
            std::env::temp_dir().join("auralis-memory-artifacts"),
        ));

        Ok((
            RuntimeServices {
                project_repo: project_repo.clone(),
                job_repo: job_repo.clone(),
                artifact_index: artifact_index.clone(),
                artifact_store: artifact_store.clone(),
                storage_uow: Arc::new(adapters_storage::memory::InMemoryStorageUnitOfWork::new(
                    db,
                    artifact_index,
                    artifact_store,
                )),
            },
            None, // No outbox worker in memory mode for now
        ))
    } else {
        let app_data_dir = app.path().app_data_dir()?;
        std::fs::create_dir_all(&app_data_dir)?;

        let db_path = app_data_dir.join("auralis.sqlite");

        let pool =
            tauri::async_runtime::block_on(adapters_storage::sqlite::connect_sqlite(db_path))?;

        let repo: crate::state::RuntimeProjectRepository =
            Arc::new(SqliteProjectRepository::new(pool.clone()));

        let idx: crate::state::RuntimeArtifactIndex =
            Arc::new(SqliteArtifactIndex::new(pool.clone()));

        let job_repo: Arc<dyn JobRepository> = Arc::new(SqliteJobRepository::new(pool.clone()));

        let recovery_storage =
            Arc::new(adapters_storage::sqlite::recovery::SqliteRecoveryStorage::new(pool.clone()));
        let use_case =
            application::usecases::system::recover_interrupted::usecase::RecoverInterruptedStateUseCase::new(
                recovery_storage,
            );
        let report = tauri::async_runtime::block_on(use_case.execute())?;

        for warning in &report.warnings {
            println!("Recovery warning: {:?}", warning);
        }
        for violation in &report.resolved_violations {
            println!("Recovery resolved violation: {:?}", violation);
        }

        if report.has_blocking_failures() {
            for failure in &report.persistence_failures {
                eprintln!("Recovery persistence failure: {:?}", failure);
            }
            for violation in &report.unresolved_violations {
                eprintln!("Recovery unresolved violation: {:?}", violation);
            }
            return Err("Startup halted due to fatal state recovery issues.".into());
        }

        if report.actions_applied > 0 {
            println!(
                "Recovery applied {} actions successfully.",
                report.actions_applied
            );
        }

        let artifacts_dir = app_data_dir.join("artifacts");
        std::fs::create_dir_all(&artifacts_dir)?;
        let store: crate::state::RuntimeArtifactStore =
            Arc::new(LocalArtifactStore::new(artifacts_dir));

        let outbox_repo = SqliteOutboxRepository::new(pool.clone());

        let tx_gateway: crate::state::RuntimeStorageUnitOfWork =
            Arc::new(SqliteStorageUnitOfWork::new(pool.clone()));

        Ok((
            RuntimeServices {
                project_repo: repo,
                job_repo,
                artifact_index: idx,
                artifact_store: store,
                storage_uow: tx_gateway,
            },
            Some(outbox_repo),
        ))
    }
}
