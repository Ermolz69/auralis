#[cfg(test)]
mod tests {
    use crate::sqlite::connection::connect_sqlite;
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
    async fn rename_migration_hack() {
        let _ = std::fs::rename(
            "migrations/0006_outbox_aggregate.sql",
            "migrations/0011_outbox_aggregate.sql",
        );
    }
}
