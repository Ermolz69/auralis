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
        let project_repo = Arc::new(InMemoryProjectRepository::new());
        let job_repo = Arc::new(adapters_storage::memory::InMemoryJobRepository::new());
        Ok((
            RuntimeServices {
                project_repo: project_repo.clone(),
                job_repo: job_repo.clone(),
                artifact_index: Arc::new(InMemoryArtifactIndex::new()),
                artifact_store: Arc::new(LocalArtifactStore::new(
                    std::env::temp_dir().join("auralis-memory-artifacts"),
                )),
                storage_uow: Arc::new(
                    adapters_storage::memory::InMemoryStorageUnitOfWork::new(
                        project_repo,
                        job_repo,
                    ),
                ),
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

        let use_case = application::usecases::project::recover_interrupted::RecoverInterruptedProjectsUseCase::new(repo.clone());
        tauri::async_runtime::block_on(use_case.execute())?;

        let job_use_case =
            application::usecases::job::recover_interrupted::RecoverInterruptedJobsUseCase::new(
                job_repo.clone(),
            );
        tauri::async_runtime::block_on(job_use_case.execute())?;

        let store: crate::state::RuntimeArtifactStore =
            Arc::new(LocalArtifactStore::new(app_data_dir.join("artifacts")));

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
