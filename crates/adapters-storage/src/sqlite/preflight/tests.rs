#[cfg(test)]
mod preflight_tests {
    use crate::sqlite::preflight::inspector::{
        DatabaseClassification, LegacyDatabaseInspector, LegacySchemaVersion,
    };
    use crate::sqlite::preflight::state_machine::TransitionStateMachine;
    use sqlx::SqlitePool;
    use std::path::Path;
    use tempfile::tempdir;
    use tokio::fs;

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
}
