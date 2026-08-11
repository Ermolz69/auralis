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

pub fn setup_storage(
    app_paths: &AppPaths,
    workspace_root: &std::path::Path,
) -> Result<(RuntimeServices, SqliteOutboxRepository), Box<dyn std::error::Error>> {
    match resolve_storage_backend(std::env::var("AURALIS_STORAGE").ok().as_deref())? {
        StorageBackend::Sqlite => {
            std::fs::create_dir_all(app_paths.root())?;

            let db_path = app_paths.database();

            let pool =
                tauri::async_runtime::block_on(adapters_storage::sqlite::connect_sqlite(db_path))?;

            tauri::async_runtime::block_on(
                adapters_storage::sqlite::migrations_runtime::run_runtime_backfills(
                    &pool,
                    workspace_root,
                ),
            )?;

            let repo: crate::state::RuntimeProjectRepository =
                Arc::new(SqliteProjectRepository::new(pool.clone()));

            let idx: crate::state::RuntimeArtifactIndex =
                Arc::new(SqliteArtifactIndex::new(pool.clone()));

            let sqlite_job_repo = Arc::new(SqliteJobRepository::new(pool.clone()));
            let job_repo: Arc<dyn JobRepository> = sqlite_job_repo.clone();
            let job_query: Arc<dyn ports::job_query::JobQueryPort> = sqlite_job_repo;

            let recovery_storage = Arc::new(
                adapters_storage::sqlite::recovery::SqliteRecoveryStorage::new(pool.clone()),
            );
            let use_case =
            application::usecases::system::recover_interrupted::usecase::RecoverInterruptedStateUseCase::new(
                recovery_storage,
            );
            let report = tauri::async_runtime::block_on(use_case.execute())?;

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

            let artifacts_dir = app_paths.projects();
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
                    job_query,
                    artifact_index: idx,
                    artifact_store: store,
                    storage_uow: tx_gateway,
                },
                outbox_repo,
            ))
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use application::usecases::system::recover_interrupted::report::{
        PersistenceFailure, RecoveryActionType, RecoveryReport,
    };
    use domain::system::recovery::{RecoveryIssueType, RecoveryViolation};
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

    #[test]
    fn storage_backend_config_accepts_default_and_sqlite_only() {
        assert_eq!(
            resolve_storage_backend(None).unwrap(),
            StorageBackend::Sqlite
        );
        assert_eq!(
            resolve_storage_backend(Some("")).unwrap(),
            StorageBackend::Sqlite
        );
        assert_eq!(
            resolve_storage_backend(Some("sqlite")).unwrap(),
            StorageBackend::Sqlite
        );
    }

    #[test]
    fn storage_backend_config_rejects_unknown_and_legacy_memory() {
        assert_eq!(
            resolve_storage_backend(Some("postgres")).unwrap_err(),
            UnsupportedStorageBackend
        );
        assert_eq!(
            resolve_storage_backend(Some("in-memory")).unwrap_err(),
            UnsupportedStorageBackend
        );
    }

    #[test]
    fn production_storage_module_does_not_import_memory_adapter() {
        let source = include_str!("storage.rs");
        let production_source = source.split("#[cfg(test)]").next().unwrap();
        assert!(!production_source.contains("adapters_storage::memory"));
        assert!(!production_source.contains("InMemoryProjectRepository"));
        assert!(!production_source.contains("InMemoryArtifactIndex"));
        assert!(!production_source.contains("InMemoryStorageUnitOfWork"));
    }

    #[derive(Clone)]
    struct MockWriter {
        buf: Arc<Mutex<Vec<u8>>>,
    }

    impl<'a> MakeWriter<'a> for MockWriter {
        type Writer = Self;
        fn make_writer(&self) -> Self::Writer {
            self.clone()
        }
    }

    impl std::io::Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.buf.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_recovery_logging_redaction() {
        let mut report = RecoveryReport::new();
        report.add_persistence_failure(PersistenceFailure {
            action_kind: RecoveryActionType::FailInterruptedPair,
            project_id: None,
            job_id: None,
            error_type: "DB_ERROR".to_string(),
            message: "sqlx::Error::Database(C:\\Users\\secret\\video.mp4 token=SECRET Bearer token SELECT * FROM projects)".to_string(),
        });
        report.unresolved_violations.push(RecoveryViolation {
            project_id: None,
            job_id: None,
            issue_type: RecoveryIssueType::OrphanActiveJob,
            message: "Failed/secret/path token=XYZ Bearer BearerToken".to_string(),
        });

        let buf = Arc::new(Mutex::new(Vec::new()));
        let writer = MockWriter { buf: buf.clone() };
        let subscriber = tracing_subscriber::fmt()
            .with_writer(writer)
            .with_ansi(false)
            .finish();

        tracing::subscriber::with_default(subscriber, || {
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
            }
        });

        let logs = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
        assert!(logs.contains("RecoveryPersistenceFailure"));
        assert!(logs.contains("RecoveryUnresolvedViolation"));
        assert!(logs.contains("persistence"));
        assert!(logs.contains("state_violation"));
        assert!(logs.contains("startup_recovery"));
        assert!(logs.contains("count=1"));

        assert!(!logs.contains("secret"));
        assert!(!logs.contains("SECRET"));
        assert!(!logs.contains("token"));
        assert!(!logs.contains("Bearer"));
        assert!(!logs.contains("sqlx"));
        assert!(!logs.contains("SELECT"));
    }
}
