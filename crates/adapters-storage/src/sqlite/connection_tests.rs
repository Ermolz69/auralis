#[cfg(test)]
mod tests {
    use crate::sqlite::connection::{connect_sqlite, create_pool};
    use ports::error::PortError;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_fresh_db_migration() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("fresh.sqlite");

        let pool = connect_sqlite(&db_path).await.expect("Failed to connect");

        // Ensure _sqlx_migrations exists
        let has_migrations: bool = sqlx::query(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap()
        .is_some();
        assert!(has_migrations);
    }

    #[tokio::test]
    async fn test_migrated_db_opens_normally() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("migrated.sqlite");

        {
            let _pool = connect_sqlite(&db_path).await.unwrap();
        }

        // Second open should work normally
        let _pool = connect_sqlite(&db_path).await.unwrap();
    }

    #[tokio::test]
    async fn test_handcrafted_db_is_backed_up() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("handcrafted.sqlite");

        // Create handcrafted db
        {
            let pool = create_pool(&db_path).await.unwrap();
            sqlx::query("CREATE TABLE projects (id TEXT PRIMARY KEY)")
                .execute(&pool)
                .await
                .unwrap();
            pool.close().await;
        }

        // Connect should detect and backup
        let _pool = connect_sqlite(&db_path).await.unwrap();

        // Check backup exists
        let entries = std::fs::read_dir(dir.path()).unwrap();
        let mut has_backup = false;
        let mut has_new_db = false;
        for entry in entries {
            let name = entry.unwrap().file_name().to_string_lossy().to_string();
            if name.contains("handcrafted.sqlite.backup_") {
                has_backup = true;
            } else if name == "handcrafted.sqlite" {
                has_new_db = true;
            }
        }

        assert!(has_backup, "Backup file should be created");
        assert!(has_new_db, "New database should be created");
    }

    #[tokio::test]
    async fn test_unknown_schema_errors_out() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("unknown.sqlite");

        // Create unknown schema
        {
            let pool = create_pool(&db_path).await.unwrap();
            sqlx::query("CREATE TABLE other_table (id TEXT PRIMARY KEY)")
                .execute(&pool)
                .await
                .unwrap();
            pool.close().await;
        }

        // Connect should fail
        let result = connect_sqlite(&db_path).await;
        assert!(result.is_err());
        let err_msg = match result {
            Err(PortError::Unexpected { message }) => message,
            _ => panic!("Expected unexpected error"),
        };
        assert!(err_msg.contains("Unknown database schema detected"));
    }
}
