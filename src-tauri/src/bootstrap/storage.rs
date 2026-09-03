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
) -> Result<(RuntimeServices, SqliteOutboxRepository), Box<dyn std::error::Error>> {
    match resolve_storage_backend(std::env::var("AURALIS_STORAGE").ok().as_deref())? {
        StorageBackend::Sqlite => tauri::async_runtime::block_on(setup_storage_at_paths(app_paths)),
    }
}

pub async fn setup_storage_at_paths(
    app_paths: &AppPaths,
) -> Result<(RuntimeServices, SqliteOutboxRepository), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(app_paths.root())?;

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
            artifact_store: store,
            storage_uow: tx_gateway,
        },
        outbox_repo,
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use application::usecases::system::recover_interrupted::report::{
        PersistenceFailure, RecoveryActionType, RecoveryReport,
    };
    use domain::system::recovery::{RecoveryIssueType, RecoveryViolation};
    use std::path::{Path, PathBuf};
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
    fn storage_backend_config_rejects_unsupported_values() {
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
        assert!(production_source.contains("LocalArtifactStore::new(app_paths.projects())"));
    }

    #[tokio::test]
    async fn setup_on_empty_root_creates_database_and_projects() {
        let sandbox = tempfile::tempdir().unwrap();
        let app_root = sandbox.path().join("app-data");
        let paths = AppPaths::new(app_root.clone());

        assert!(!app_root.exists());
        let _storage = setup_storage_at_paths(&paths).await.unwrap();

        assert!(paths.database().is_file());
        assert!(paths.projects().is_dir());
        assert!(paths.projects().join(".staging").is_dir());
    }

    #[tokio::test]
    async fn setup_is_repeatable_for_the_same_paths() {
        let sandbox = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(sandbox.path().join("app-data"));

        let first = setup_storage_at_paths(&paths).await.unwrap();
        drop(first);
        let second = setup_storage_at_paths(&paths).await;

        assert!(second.is_ok());
        assert!(paths.database().is_file());
        assert!(paths.projects().is_dir());
    }

    #[tokio::test]
    async fn artifact_store_uses_projects_as_its_root() {
        let sandbox = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(sandbox.path().join("app-data"));
        let (services, _outbox) = setup_storage_at_paths(&paths).await.unwrap();
        let source = sandbox.path().join("source.mp4");
        tokio::fs::write(&source, b"video").await.unwrap();
        let project_id = domain::project::ProjectId::new();

        let staged = services
            .artifact_store
            .import_external_file(
                &project_id,
                domain::media::ArtifactKind::SourceVideo,
                &source,
                Some("source.mp4"),
            )
            .await
            .unwrap();
        services
            .artifact_store
            .finalize_staged_artifact(&staged.staging_key, &staged.final_key)
            .await
            .unwrap();

        assert!(staged
            .final_key
            .starts_with(&format!("{project_id}/source-video/")));
        assert!(paths.projects().join(&staged.final_key).is_file());
        assert!(!paths.root().join(&staged.final_key).exists());
    }

    #[tokio::test]
    async fn unavailable_app_root_fails_without_writing_to_other_roots() {
        let sandbox = tempfile::tempdir().unwrap();
        let blocked_root = sandbox.path().join("app-data");
        std::fs::write(&blocked_root, b"not a directory").unwrap();
        let paths = AppPaths::new(blocked_root.clone());

        let observed_roots = unexpected_output_roots(sandbox.path());
        let before = observed_roots
            .iter()
            .map(|path| (path.clone(), path_state(path)))
            .collect::<Vec<_>>();

        let result = setup_storage_at_paths(&paths).await;

        assert!(result.is_err());
        assert!(blocked_root.is_file());
        assert!(!sandbox.path().join("auralis.sqlite").exists());
        assert!(!sandbox.path().join("projects").exists());
        for (path, state) in before {
            assert_eq!(path_state(&path), state, "unexpected write at {path:?}");
        }

        let source = include_str!("storage.rs");
        let production_source = source.split("#[cfg(test)]").next().unwrap();
        assert!(!production_source.contains("current_dir("));
        assert!(!production_source.contains("temp_dir("));
    }

    fn unexpected_output_roots(sandbox: &Path) -> Vec<PathBuf> {
        let mut roots = vec![
            sandbox.join("auralis.sqlite"),
            sandbox.join("projects"),
            std::env::current_dir().unwrap().join("auralis.sqlite"),
            std::env::current_dir().unwrap().join("projects"),
            std::env::temp_dir().join("auralis.sqlite"),
            std::env::temp_dir().join("projects"),
        ];

        if let Some(executable_dir) = std::env::current_exe()
            .unwrap()
            .parent()
            .map(Path::to_path_buf)
        {
            roots.push(executable_dir.join("auralis.sqlite"));
            roots.push(executable_dir.join("projects"));
        }

        roots.sort();
        roots.dedup();
        roots
    }

    #[derive(Debug, PartialEq, Eq)]
    struct PathState {
        exists: bool,
        is_file: bool,
        is_dir: bool,
        len: Option<u64>,
        modified: Option<std::time::SystemTime>,
    }

    fn path_state(path: &Path) -> PathState {
        match std::fs::metadata(path) {
            Ok(metadata) => PathState {
                exists: true,
                is_file: metadata.is_file(),
                is_dir: metadata.is_dir(),
                len: Some(metadata.len()),
                modified: metadata.modified().ok(),
            },
            Err(_) => PathState {
                exists: false,
                is_file: false,
                is_dir: false,
                len: None,
                modified: None,
            },
        }
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
