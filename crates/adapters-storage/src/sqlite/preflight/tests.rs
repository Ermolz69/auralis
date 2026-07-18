#![allow(clippy::unwrap_used, clippy::expect_used)]
#[cfg(test)]
mod preflight_tests {
    use crate::sqlite::preflight::error::DatabaseTransitionError;
    use crate::sqlite::preflight::inspector::{
        DatabaseClassification, LegacyDatabaseInspector, LegacySchemaVersion,
    };
    use crate::sqlite::preflight::lock::{TRANSITION_LOCK_STALE_AFTER_SECS, TransitionLockData};
    use crate::sqlite::preflight::manifest::{TransitionManifest, TransitionStage};
    use crate::sqlite::preflight::state_machine::{TransitionFailpoint, TransitionStateMachine};
    use std::path::Path;
    use tempfile::tempdir;
    use tokio::fs;
    use uuid::Uuid;

    async fn create_handcrafted_v0(db_path: &Path) {
        let options = sqlx::sqlite::SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .connect_with(options)
            .await
            .unwrap();

        sqlx::query(
            "CREATE TABLE projects (
                id TEXT PRIMARY KEY NOT NULL,
                title TEXT NOT NULL,
                status TEXT NOT NULL,
                artifacts_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Insert a test row
        sqlx::query("INSERT INTO projects (id, title, status, artifacts_json, created_at, updated_at) VALUES ('123', 'Test Project', 'draft', '[]', '2026', '2026')")
            .execute(&pool)
            .await.unwrap();

        pool.close().await;
    }

    #[tokio::test]
    async fn test_inspector_identifies_handcrafted_v0() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("v0.sqlite");
        create_handcrafted_v0(&db_path).await;

        let classification = LegacyDatabaseInspector::inspect(&db_path).await.unwrap();
        assert_eq!(
            classification,
            DatabaseClassification::KnownLegacy(LegacySchemaVersion::HandcraftedV0)
        );
    }

    #[tokio::test]
    async fn test_inspector_identifies_unknown() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("unknown.sqlite");

        {
            let pool = sqlx::SqlitePool::connect(&format!(
                "sqlite:{}?mode=rwc",
                db_path.to_string_lossy()
            ))
            .await
            .unwrap();
            sqlx::query("CREATE TABLE something_else (id INTEGER PRIMARY KEY)")
                .execute(&pool)
                .await
                .unwrap();
            pool.close().await;
        }

        let mtime_before = fs::metadata(&db_path).await.unwrap().modified().unwrap();
        let hash_before = fs::read(&db_path).await.unwrap();

        let classification = LegacyDatabaseInspector::inspect(&db_path).await.unwrap();
        assert_eq!(classification, DatabaseClassification::Unknown);

        let mtime_after = fs::metadata(&db_path).await.unwrap().modified().unwrap();
        let hash_after = fs::read(&db_path).await.unwrap();

        assert_eq!(mtime_before, mtime_after);
        assert_eq!(hash_before, hash_after);
    }

    #[tokio::test]
    async fn test_inspector_almost_v0_is_unknown() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("almost_v0.sqlite");

        {
            let pool = sqlx::SqlitePool::connect(&format!(
                "sqlite:{}?mode=rwc",
                db_path.to_string_lossy()
            ))
            .await
            .unwrap();
            // Missing `artifacts_json`
            sqlx::query(
                "CREATE TABLE projects (
                    id TEXT PRIMARY KEY NOT NULL,
                    title TEXT NOT NULL,
                    status TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                )",
            )
            .execute(&pool)
            .await
            .unwrap();
            pool.close().await;
        }

        let classification = LegacyDatabaseInspector::inspect(&db_path).await.unwrap();
        assert_eq!(classification, DatabaseClassification::Unknown);
    }

    #[tokio::test]
    async fn test_full_state_machine_transition() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("transition.sqlite");
        create_handcrafted_v0(&db_path).await;

        let sm = TransitionStateMachine::new(&db_path);
        sm.run().await.expect("Transition should succeed");

        // The original database should have been replaced with a ManagedSqlx one
        let classification = LegacyDatabaseInspector::inspect(&db_path).await.unwrap();
        assert_eq!(classification, DatabaseClassification::ManagedSqlx);

        // Check that backup exists and contains data
        let mut backup_dir = None;
        let mut entries = fs::read_dir(dir.path().join("backups")).await.unwrap();
        while let Some(entry) = entries.next_entry().await.unwrap() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("auralis-") {
                backup_dir = Some(entry.path());
                break;
            }
        }

        let backup_dir = backup_dir.expect("Backup directory should exist");
        let backup_db_path = backup_dir.join("transition.sqlite");

        let ro_pool = sqlx::SqlitePool::connect(&format!(
            "sqlite:{}?mode=ro",
            backup_db_path.to_string_lossy()
        ))
        .await
        .unwrap();
        let title: String = sqlx::query_scalar("SELECT title FROM projects WHERE id='123'")
            .fetch_one(&ro_pool)
            .await
            .unwrap();
        assert_eq!(title, "Test Project");
        ro_pool.close().await;

        // Check that manifest exists
        assert!(backup_dir.join("manifest.json").exists());

        // Quarantine directory should be gone
        let mut has_quarantine = false;
        let mut entries = fs::read_dir(dir.path().join("backups")).await.unwrap();
        while let Some(entry) = entries.next_entry().await.unwrap() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("quarantine-") {
                has_quarantine = true;
            }
        }
        assert!(!has_quarantine, "Quarantine directory should be cleaned up");
    }

    #[tokio::test]
    async fn crash_matrix_resumes_from_every_durable_stage() {
        for failpoint in [
            TransitionFailpoint::Started,
            TransitionFailpoint::BackupFinalized,
            TransitionFailpoint::NewDatabaseReady,
            TransitionFailpoint::OldDatabaseQuarantined,
            TransitionFailpoint::NewDatabaseActivated,
        ] {
            let dir = tempdir().unwrap();
            let db_path = dir
                .path()
                .join(format!("transition_{:?}.sqlite", failpoint));
            create_handcrafted_v0(&db_path).await;

            let sm = TransitionStateMachine::new(&db_path);
            assert!(sm.run_with_failpoint(failpoint).await.is_err());

            let sm = TransitionStateMachine::new(&db_path);
            sm.run().await.expect("startup resume should complete");

            let classification = LegacyDatabaseInspector::inspect(&db_path).await.unwrap();
            assert_eq!(classification, DatabaseClassification::ManagedSqlx);
            assert_backup_available(dir.path(), db_path.file_name().unwrap().to_str().unwrap())
                .await;
            assert!(!manifest_path(dir.path()).exists());
        }
    }

    #[tokio::test]
    async fn live_lock_is_not_stolen() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("transition.sqlite");
        create_handcrafted_v0(&db_path).await;
        write_lock(
            &db_path,
            std::process::id(),
            now_sec(),
            dir.path().join("backups").join("transition_manifest.json"),
        )
        .await;

        let err = TransitionStateMachine::new(&db_path)
            .run()
            .await
            .unwrap_err();
        assert!(matches!(err, DatabaseTransitionError::LiveTransitionLock));
    }

    #[tokio::test]
    async fn dead_lock_is_reclaimed() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("transition.sqlite");
        create_handcrafted_v0(&db_path).await;
        write_lock(
            &db_path,
            999_999,
            now_sec() - TRANSITION_LOCK_STALE_AFTER_SECS - 1,
            dir.path().join("backups").join("transition_manifest.json"),
        )
        .await;

        TransitionStateMachine::new(&db_path).run().await.unwrap();
        assert_eq!(
            LegacyDatabaseInspector::inspect(&db_path).await.unwrap(),
            DatabaseClassification::ManagedSqlx
        );
    }

    #[tokio::test]
    async fn corrupt_lock_and_manifest_are_typed_failures() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("transition.sqlite");
        create_handcrafted_v0(&db_path).await;
        fs::write(db_path.with_extension("sqlite.lock"), b"not-json")
            .await
            .unwrap();
        let err = TransitionStateMachine::new(&db_path)
            .run()
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            DatabaseTransitionError::CorruptTransitionLock(_)
        ));

        fs::remove_file(db_path.with_extension("sqlite.lock"))
            .await
            .unwrap();
        fs::create_dir_all(dir.path().join("backups"))
            .await
            .unwrap();
        fs::write(manifest_path(dir.path()), b"{bad").await.unwrap();
        let err = TransitionStateMachine::new(&db_path)
            .run()
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            DatabaseTransitionError::CorruptTransitionState(_)
        ));
    }

    #[tokio::test]
    async fn missing_db_with_manifest_is_resume_mismatch() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("transition.sqlite");
        let manifest = test_manifest(dir.path(), "transition.sqlite", TransitionStage::Started);
        manifest.save(&manifest_path(dir.path())).await.unwrap();

        let err = TransitionStateMachine::new(&db_path)
            .run()
            .await
            .unwrap_err();
        assert!(matches!(err, DatabaseTransitionError::ResumeMismatch(_)));
    }

    #[tokio::test]
    async fn manifest_path_validation_rejects_escape_and_bad_names() {
        let dir = tempdir().unwrap();
        let backup_root = dir.path().join("backups");
        let db_root = dir.path();

        let mut manifest = test_manifest(dir.path(), "transition.sqlite", TransitionStage::Started);
        manifest.database_name = "../escape.sqlite".to_string();
        assert!(matches!(
            manifest.validate(db_root, &backup_root),
            Err(DatabaseTransitionError::CorruptTransitionState(_))
        ));

        let mut manifest = test_manifest(dir.path(), "transition.sqlite", TransitionStage::Started);
        manifest.backup_directory = backup_root.join("..").join("outside");
        assert!(matches!(
            manifest.validate(db_root, &backup_root),
            Err(DatabaseTransitionError::CorruptTransitionState(_))
        ));
    }

    #[tokio::test]
    async fn relative_path_without_filename_is_rejected() {
        let err = TransitionStateMachine::new(Path::new(""))
            .run()
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            DatabaseTransitionError::CorruptTransitionState(_)
        ));
    }

    async fn assert_backup_available(root: &Path, database_name: &str) {
        let mut backup_dir = None;
        let mut entries = fs::read_dir(root.join("backups")).await.unwrap();
        while let Some(entry) = entries.next_entry().await.unwrap() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("auralis-") {
                backup_dir = Some(entry.path());
                break;
            }
        }
        let backup_db_path = backup_dir.unwrap().join(database_name);
        let pool = sqlx::SqlitePool::connect(&format!(
            "sqlite:{}?mode=ro",
            backup_db_path.to_string_lossy()
        ))
        .await
        .unwrap();
        let title: String = sqlx::query_scalar("SELECT title FROM projects WHERE id='123'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(title, "Test Project");
        pool.close().await;
    }

    async fn write_lock(
        db_path: &Path,
        pid: u32,
        timestamp_sec: u64,
        manifest_path: std::path::PathBuf,
    ) {
        let lock = TransitionLockData {
            operation_id: Uuid::new_v4(),
            pid,
            timestamp_sec,
            manifest_path,
        };
        fs::write(
            db_path.with_extension("sqlite.lock"),
            serde_json::to_vec(&lock).unwrap(),
        )
        .await
        .unwrap();
    }

    #[rustfmt::skip]
    fn test_manifest(root: &Path, database_name: &str, stage: TransitionStage) -> TransitionManifest {
        let operation_id = Uuid::new_v4();
        TransitionManifest {
            operation_id,
            legacy_version: LegacySchemaVersion::HandcraftedV0,
            stage,
            database_name: database_name.to_string(),
            backup_directory: root.join("backups").join(format!("auralis-{}", operation_id)),
            new_database_name: format!("{}.new-{}", database_name, operation_id),
            quarantine_directory: root.join("backups").join(format!("quarantine-{}", operation_id)),
        }
    }

    fn manifest_path(root: &Path) -> std::path::PathBuf {
        root.join("backups").join("transition_manifest.json")
    }

    #[rustfmt::skip]
    fn now_sec() -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
    }
}
