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
    workspace_root: &std::path::Path,
) -> Result<(RuntimeServices, Option<SqliteOutboxRepository>), Box<dyn std::error::Error>> {
    if std::env::var("AURALIS_STORAGE").unwrap_or_default() == "in-memory" {
        tracing::warn!("Running with IN-MEMORY storage adapter! Data will be lost on exit.");
        let db = Arc::new(std::sync::Mutex::new(
            adapters_storage::memory::InMemoryDatabase::new(),
        ));
        let project_repo = Arc::new(InMemoryProjectRepository::new(db.clone()));
        let memory_job_repo = Arc::new(adapters_storage::memory::InMemoryJobRepository::new(
            db.clone(),
        ));
        let job_repo: Arc<dyn JobRepository> = memory_job_repo.clone();
        let job_query: Arc<dyn ports::job_query::JobQueryPort> = memory_job_repo;
        let artifact_index = Arc::new(InMemoryArtifactIndex::new());
        let artifact_store = Arc::new(LocalArtifactStore::new(
            std::env::temp_dir().join("auralis-memory-artifacts"),
        ));

        Ok((
            RuntimeServices {
                project_repo: project_repo.clone(),
                job_repo,
                job_query,
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

        let recovery_storage =
            Arc::new(adapters_storage::sqlite::recovery::SqliteRecoveryStorage::new(pool.clone()));
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
                job_query,
                artifact_index: idx,
                artifact_store: store,
                storage_uow: tx_gateway,
            },
            Some(outbox_repo),
        ))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use application::usecases::system::recover_interrupted::report::{
        PersistenceFailure, RecoveryActionType, RecoveryReport,
    };
    use domain::system::recovery::{RecoveryIssueType, RecoveryViolation};
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

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
                        count = report.unresolved_violations.len(),
                        "Recovery unresolved violation occurred"
                    );
                }
            }
        });

        let logs = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
        assert!(logs.contains("RecoveryPersistenceFailure"));
        assert!(logs.contains("RecoveryUnresolvedViolation"));

        assert!(!logs.contains("secret"));
        assert!(!logs.contains("SECRET"));
        assert!(!logs.contains("token"));
        assert!(!logs.contains("Bearer"));
        assert!(!logs.contains("sqlx"));
        assert!(!logs.contains("SELECT"));
    }
}
