#![allow(clippy::unwrap_used, clippy::expect_used)]
#[cfg(test)]
mod tests {
    use crate::sqlite::connection::{connect_sqlite, create_pool};
    use tempfile::tempdir;

    #[tokio::test]
    async fn fresh_database_uses_the_final_schema_only() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("fresh.sqlite");

        let pool = connect_sqlite(&db_path).await.expect("Failed to connect");

        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(tables, ["artifacts", "jobs", "outbox_messages", "projects"]);

        let project_columns: Vec<String> =
            sqlx::query_scalar("SELECT name FROM pragma_table_info('projects') ORDER BY cid")
                .fetch_all(&pool)
                .await
                .unwrap();
        assert_eq!(
            project_columns,
            [
                "id",
                "title",
                "status",
                "source_json",
                "metadata_json",
                "source_language",
                "target_language",
                "transcript_json",
                "active_job_id",
                "last_terminal_job_id",
                "created_at",
                "updated_at",
                "revision",
                "avatar_data_url",
            ]
        );

        let rejected = sqlx::query(
            "INSERT INTO artifacts (id, project_id, kind, location_kind, location_value, created_at, updated_at) VALUES ('a', 'missing', 'SourceVideo', 'LocalPath', '/tmp/a', 'now', 'now')",
        )
        .execute(&pool)
        .await;
        assert!(rejected.is_err());
    }

    #[tokio::test]
    async fn final_database_opens_normally_more_than_once() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("final.sqlite");

        {
            let _pool = connect_sqlite(&db_path).await.unwrap();
        }

        // Second open should work normally
        let _pool = connect_sqlite(&db_path).await.unwrap();
    }

    #[tokio::test]
    async fn database_with_another_schema_is_rejected_without_conversion() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("unsupported.sqlite");
        let pool = create_pool(&db_path).await.unwrap();
        sqlx::query("CREATE TABLE unrelated_state (id TEXT PRIMARY KEY)")
            .execute(&pool)
            .await
            .unwrap();
        pool.close().await;

        let error = connect_sqlite(&db_path).await.unwrap_err();
        assert!(matches!(
            error,
            ports::error::PortError::Storage {
                operation: "validate_sqlite_schema",
                ..
            }
        ));
    }
}
