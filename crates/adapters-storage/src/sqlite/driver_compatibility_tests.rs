#![allow(clippy::unwrap_used)]

use super::{SqliteProjectRepository, connect_sqlite};
use chrono::{DateTime, Utc};
use domain::project::Project;
use ports::repository::ProjectRepository;

#[tokio::test]
async fn every_pooled_connection_enforces_foreign_keys_and_wal() {
    let directory = tempfile::tempdir().unwrap();
    let pool = connect_sqlite(directory.path().join("pool.sqlite"))
        .await
        .unwrap();
    let mut connections = Vec::new();
    for _ in 0..5 {
        let mut connection = pool.acquire().await.unwrap();
        let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&mut *connection)
            .await
            .unwrap();
        let journal: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(&mut *connection)
            .await
            .unwrap();
        assert_eq!(foreign_keys, 1);
        assert_eq!(journal, "wal");
        let invalid_reference = sqlx::query(
            "INSERT INTO artifacts (id, project_id, kind, location_kind, location_value, state, created_at, updated_at) VALUES ('orphan', 'missing-project', 'source_video', 'StorageKey', 'test/orphan.mp4', 'pending_finalize', '2026-09-03T00:00:00Z', '2026-09-03T00:00:00Z')",
        )
        .execute(&mut *connection)
        .await
        .unwrap_err();
        assert!(
            invalid_reference
                .as_database_error()
                .unwrap()
                .is_foreign_key_violation(),
            "Expected a foreign-key violation, got {invalid_reference}"
        );
        connections.push(connection);
    }
    drop(connections);
    pool.close().await;
}

#[tokio::test]
async fn bound_text_and_chrono_values_survive_commit_rollback_and_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("round-trip.sqlite");
    let pool = connect_sqlite(&path).await.unwrap();
    let project = Project::new("SQLite 'quotes' — 日本語 🎬".into()).unwrap();
    SqliteProjectRepository::new(pool.clone())
        .create(project.clone())
        .await
        .unwrap();
    let timestamp: DateTime<Utc> = "2026-09-03T12:34:56.123456789Z".parse().unwrap();
    let mut transaction = pool.begin().await.unwrap();
    sqlx::query("UPDATE projects SET updated_at = ? WHERE id = ?")
        .bind(timestamp)
        .bind(project.id().to_string())
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    let mut transaction = pool.begin().await.unwrap();
    sqlx::query("UPDATE projects SET title = ? WHERE id = ?")
        .bind("rolled back")
        .bind(project.id().to_string())
        .execute(&mut *transaction)
        .await
        .unwrap();
    drop(transaction);
    pool.close().await;

    let pool = connect_sqlite(&path).await.unwrap();
    let restored = SqliteProjectRepository::new(pool.clone())
        .get(project.id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(restored.title(), project.title());
    assert_eq!(restored.updated_at(), timestamp);
    assert_eq!(restored.revision(), project.revision());
    let decoded: DateTime<Utc> = sqlx::query_scalar("SELECT updated_at FROM projects WHERE id = ?")
        .bind(project.id().to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(decoded, timestamp);
    let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(integrity, "ok");
    pool.close().await;
}
