use super::paths::AppPaths;
use super::services::RuntimeServices;
use adapters_storage::local::artifact_store::LocalArtifactStore;
use adapters_storage::sqlite::{
    SqliteArtifactIndex, SqliteJobRepository, SqliteOutboxRepository, SqliteProjectRepository,
    SqliteStorageUnitOfWork,
};
use ports::repository::JobRepository;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StorageBackend {
    Sqlite,
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
#[error("unsupported storage backend; use sqlite or leave AURALIS_STORAGE unset")]
struct UnsupportedStorageBackend;

fn resolve_storage_backend(
    value: Option<&str>,
) -> Result<StorageBackend, UnsupportedStorageBackend> {
    match value.map(str::trim).filter(|value| !value.is_empty()) {
        None | Some("sqlite") => Ok(StorageBackend::Sqlite),
        Some(_) => Err(UnsupportedStorageBackend),
    }
}

pub(crate) fn setup_storage(
    app_paths: &AppPaths,
    storage_lease: Arc<super::app_root_lease::AppRootLease>,
) -> Result<(RuntimeServices, SqliteOutboxRepository), Box<dyn std::error::Error>> {
    match resolve_storage_backend(std::env::var("AURALIS_STORAGE").ok().as_deref())? {
        StorageBackend::Sqlite => {
            tauri::async_runtime::block_on(setup_storage_with_lease(app_paths, storage_lease))
        }
    }
}

#[cfg(test)]
pub async fn setup_storage_at_paths(
    app_paths: &AppPaths,
) -> Result<(RuntimeServices, SqliteOutboxRepository), Box<dyn std::error::Error>> {
    let storage_lease = Arc::new(super::app_root_lease::AppRootLease::acquire(
        app_paths.root(),
    )?);
    setup_storage_with_lease(app_paths, storage_lease).await
}

async fn setup_storage_with_lease(
    app_paths: &AppPaths,
    storage_lease: Arc<super::app_root_lease::AppRootLease>,
) -> Result<(RuntimeServices, SqliteOutboxRepository), Box<dyn std::error::Error>> {
    let pool = adapters_storage::sqlite::connect_sqlite(app_paths.database()).await?;

    let repo: crate::state::RuntimeProjectRepository =
        Arc::new(SqliteProjectRepository::new(pool.clone()));

    let idx: crate::state::RuntimeArtifactIndex = Arc::new(SqliteArtifactIndex::new(pool.clone()));

    let sqlite_job_repo = Arc::new(SqliteJobRepository::new(pool.clone()));
    let job_repo: Arc<dyn JobRepository> = sqlite_job_repo.clone();
    let job_query: Arc<dyn ports::job_query::JobQueryPort> = sqlite_job_repo;

    let recovery_storage =
        Arc::new(adapters_storage::sqlite::recovery::SqliteRecoveryStorage::new(pool.clone()));
    let use_case =
        application::usecases::system::recover_interrupted::usecase::RecoverInterruptedStateUseCase::new(
            recovery_storage,
        );
    let report = use_case.execute().await?;

    if !report.warnings.is_empty() {
        tracing::warn!(
            count = report.warnings.len(),
            "Recovery warning(s) occurred"
        );
    }
    if !report.resolved_violations.is_empty() {
        tracing::info!(
            count = report.resolved_violations.len(),
            "Recovery resolved violation(s)"
        );
    }

    if report.has_blocking_failures() {
        if !report.persistence_failures.is_empty() {
            tracing::error!(
                error = %common::observability::redaction::DiagnosticError {
                    kind: "RecoveryPersistenceFailure",
                    code: None,
                    retryable: false,
                },
                category = "persistence",
                action = "startup_recovery",
                count = report.persistence_failures.len(),
                "Recovery persistence failure occurred"
            );
        }
        if !report.unresolved_violations.is_empty() {
            tracing::error!(
                error = %common::observability::redaction::DiagnosticError {
                    kind: "RecoveryUnresolvedViolation",
                    code: None,
                    retryable: false,
                },
                category = "state_violation",
                action = "startup_recovery",
                count = report.unresolved_violations.len(),
                "Recovery unresolved violation occurred"
            );
        }
        return Err("Startup halted due to fatal state recovery issues.".into());
    }

    if report.actions_applied > 0 {
        tracing::info!(
            "Recovery applied {} actions successfully.",
            report.actions_applied
        );
    }

    std::fs::create_dir_all(app_paths.projects().join(".staging"))?;
    let store: crate::state::RuntimeArtifactStore =
        Arc::new(LocalArtifactStore::new(app_paths.projects()));

    let outbox_repo = SqliteOutboxRepository::new(pool.clone());

    let tx_gateway: crate::state::RuntimeStorageUnitOfWork =
        Arc::new(SqliteStorageUnitOfWork::new(pool.clone()));

    Ok((
        RuntimeServices {
            ui_preferences: Arc::new(
                adapters_storage::sqlite::ui_preferences::SqliteUiPreferences::new(pool.clone()),
            ),
            _storage_lease: storage_lease,
            youtube_imports: Arc::new(
                adapters_storage::sqlite::youtube_import_journal::SqliteYoutubeImportJournal::new(
                    pool.clone(),
                ),
            ),
            project_avatar_repo: Arc::new(
                adapters_storage::sqlite::SqliteProjectAvatarRepository::new(pool.clone()),
            ),
            project_repo: repo,
            job_repo,
            job_query,
            artifact_index: idx,
            artifact_finalization: Arc::new(SqliteArtifactIndex::new(pool.clone())),
            artifact_store: store,
            storage_uow: tx_gateway,
        },
        outbox_repo,
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[path = "storage/tests.rs"]
mod tests;
