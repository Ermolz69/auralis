use crate::sqlite::migrations_runtime::outbox_temp_paths::migrate_delete_temp_paths;
use serde_json::json;
use sqlx::Row;
use std::fs;
use tempfile::tempdir;

async fn setup_db() -> sqlx::SqlitePool {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
        r#"
        CREATE TABLE outbox_messages (
            id TEXT PRIMARY KEY,
            kind TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            status TEXT NOT NULL,
            attempts INTEGER NOT NULL,
            next_attempt_at TEXT NOT NULL,
            locked_at TEXT,
            locked_by TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            last_error TEXT
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    pool
}

async fn insert_legacy_message(pool: &sqlx::SqlitePool, id: &str, payload: serde_json::Value) {
    sqlx::query(
        r#"
        INSERT INTO outbox_messages (
            id, kind, payload_json, status, attempts, next_attempt_at, created_at, updated_at
        ) VALUES (?, 'delete_temp_path', ?, 'pending', 0, 'now', 'now', 'now')
        "#,
    )
    .bind(id)
    .bind(payload.to_string())
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn legacy_path_inside_root() {
    let pool = setup_db().await;
    let root_dir = tempdir().unwrap();
    let root_path = root_dir.path();

    // Create a real directory
    let tmp_dir = root_path.join("tmp");
    fs::create_dir(&tmp_dir).unwrap();

    let target = tmp_dir.join("some_file.txt");
    // Don't need to create the file itself for nearest-ancestor

    let payload = json!({
        "type": "delete_temp_path",
        "absolute_path": target.to_str().unwrap()
    });
    insert_legacy_message(&pool, "msg-1", payload).await;

    let mut tx = pool.begin().await.unwrap();
    migrate_delete_temp_paths(&mut tx, root_path).await.unwrap();
    tx.commit().await.unwrap();

    let row =
        sqlx::query("SELECT kind, payload_json, status FROM outbox_messages WHERE id = 'msg-1'")
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(row.get::<String, _>("kind"), "delete_workspace_file");
    assert_eq!(row.get::<String, _>("status"), "pending");

    let payload_str: String = row.get("payload_json");
    let migrated: serde_json::Value = serde_json::from_str(&payload_str).unwrap();

    assert_eq!(migrated["type"], "delete_workspace_file");
    assert_eq!(migrated["workspace_key"], "tmp/some_file.txt");
}

#[tokio::test]
async fn legacy_path_outside_root() {
    let pool = setup_db().await;
    let root_dir = tempdir().unwrap();
    let outside_dir = tempdir().unwrap();

    let payload = json!({
        "type": "delete_temp_path",
        "path": outside_dir.path().join("file.txt").to_str().unwrap()
    });
    insert_legacy_message(&pool, "msg-1", payload).await;

    let mut tx = pool.begin().await.unwrap();
    migrate_delete_temp_paths(&mut tx, root_dir.path())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let row = sqlx::query("SELECT status, last_error FROM outbox_messages WHERE id = 'msg-1'")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(row.get::<String, _>("status"), "dead");
    assert_eq!(
        row.get::<String, _>("last_error"),
        "Legacy DeleteTempPath path is outside workspace root"
    );
}

#[tokio::test]
async fn legacy_sibling_prefix() {
    let pool = setup_db().await;
    let base_dir = tempdir().unwrap();
    let root_path = base_dir.path().join("workspaces");
    fs::create_dir(&root_path).unwrap();

    let evil_path = base_dir.path().join("workspaces_evil");
    fs::create_dir(&evil_path).unwrap();
    let target = evil_path.join("tmp/file.txt");

    let payload = json!({
        "type": "delete_temp_path",
        "path": target.to_str().unwrap()
    });
    insert_legacy_message(&pool, "msg-1", payload).await;

    let mut tx = pool.begin().await.unwrap();
    migrate_delete_temp_paths(&mut tx, &root_path)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let row = sqlx::query("SELECT status, last_error FROM outbox_messages WHERE id = 'msg-1'")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(row.get::<String, _>("status"), "dead");
    assert_eq!(
        row.get::<String, _>("last_error"),
        "Legacy DeleteTempPath path is outside workspace root"
    );
}

#[tokio::test]
async fn legacy_nonexistent_path_inside_root() {
    let pool = setup_db().await;
    let root_dir = tempdir().unwrap();
    let root_path = root_dir.path();

    // Nearest ancestor exists, target does not
    let tmp_dir = root_path.join("tmp");
    fs::create_dir(&tmp_dir).unwrap();
    let target = tmp_dir.join("missing_dir").join("file.txt");

    let payload = json!({
        "type": "delete_temp_path",
        "path": target.to_str().unwrap()
    });
    insert_legacy_message(&pool, "msg-1", payload).await;

    let mut tx = pool.begin().await.unwrap();
    migrate_delete_temp_paths(&mut tx, root_path).await.unwrap();
    tx.commit().await.unwrap();

    let row =
        sqlx::query("SELECT kind, payload_json, status FROM outbox_messages WHERE id = 'msg-1'")
            .fetch_one(&pool)
            .await
            .unwrap();

    assert_eq!(row.get::<String, _>("status"), "pending");

    let payload_str: String = row.get("payload_json");
    let migrated: serde_json::Value = serde_json::from_str(&payload_str).unwrap();
    assert_eq!(migrated["workspace_key"], "tmp/missing_dir/file.txt");
}

#[cfg(unix)]
#[tokio::test]
async fn legacy_symlink_parent_escape() {
    let pool = setup_db().await;
    let base_dir = tempdir().unwrap();
    let root_path = base_dir.path().join("workspace");
    fs::create_dir(&root_path).unwrap();
    let tmp_dir = root_path.join("tmp");
    fs::create_dir(&tmp_dir).unwrap();

    let outside_path = base_dir.path().join("outside");
    fs::create_dir(&outside_path).unwrap();

    // tmp/link -> outside
    std::os::unix::fs::symlink(&outside_path, tmp_dir.join("link")).unwrap();

    let target = tmp_dir.join("link").join("missing_file.txt");

    let payload = json!({
        "type": "delete_temp_path",
        "path": target.to_str().unwrap()
    });
    insert_legacy_message(&pool, "msg-1", payload).await;

    let mut tx = pool.begin().await.unwrap();
    migrate_delete_temp_paths(&mut tx, &root_path)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let row = sqlx::query("SELECT status, last_error FROM outbox_messages WHERE id = 'msg-1'")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(row.get::<String, _>("status"), "dead");
    assert_eq!(
        row.get::<String, _>("last_error"),
        "Legacy DeleteTempPath path is outside workspace root"
    );
}

#[tokio::test]
async fn legacy_malformed_json_to_dead() {
    let pool = setup_db().await;
    let root_dir = tempdir().unwrap();

    let payload = json!({
        "type": "delete_temp_path",
        "unknown_field": 123
    });
    insert_legacy_message(&pool, "msg-1", payload).await;

    let mut tx = pool.begin().await.unwrap();
    migrate_delete_temp_paths(&mut tx, root_dir.path())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let row = sqlx::query("SELECT status, last_error FROM outbox_messages WHERE id = 'msg-1'")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(row.get::<String, _>("status"), "dead");
    assert_eq!(
        row.get::<String, _>("last_error"),
        "Legacy DeleteTempPath had no valid path fields"
    );
}

#[tokio::test]
async fn legacy_relative_path_with_dots_to_dead() {
    let pool = setup_db().await;
    let root_dir = tempdir().unwrap();
    let target = "tmp/missing?.txt";

    let payload = json!({
        "type": "delete_temp_path",
        "path": target
    });
    insert_legacy_message(&pool, "msg-1", payload).await;

    let mut tx = pool.begin().await.unwrap();
    migrate_delete_temp_paths(&mut tx, root_dir.path())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let row = sqlx::query("SELECT status, last_error FROM outbox_messages WHERE id = 'msg-1'")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(row.get::<String, _>("status"), "dead");
    assert_eq!(
        row.get::<String, _>("last_error"),
        "Legacy DeleteTempPath is not a valid WorkspaceKey"
    );
}

#[tokio::test]
async fn legacy_path_inside_root_but_outside_tmp_to_dead() {
    let pool = setup_db().await;
    let root_dir = tempdir().unwrap();

    let invalid_dir = root_dir.path().join("invalid");
    fs::create_dir(&invalid_dir).unwrap();
    let target = invalid_dir.join("file.txt");

    let payload = json!({
        "type": "delete_temp_path",
        "path": target.to_str().unwrap()
    });
    insert_legacy_message(&pool, "msg-1", payload).await;

    let mut tx = pool.begin().await.unwrap();
    migrate_delete_temp_paths(&mut tx, root_dir.path())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let row = sqlx::query("SELECT status, last_error FROM outbox_messages WHERE id = 'msg-1'")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(row.get::<String, _>("status"), "dead");
    assert_eq!(
        row.get::<String, _>("last_error"),
        "Legacy DeleteTempPath does not start with tmp/"
    );
}

#[tokio::test]
async fn legacy_migration_is_idempotent() {
    let pool = setup_db().await;
    let root_dir = tempdir().unwrap();

    let tmp_dir = root_dir.path().join("tmp");
    fs::create_dir(&tmp_dir).unwrap();
    let target = tmp_dir.join("file.txt");

    let payload = json!({
        "type": "delete_temp_path",
        "path": target.to_str().unwrap()
    });
    insert_legacy_message(&pool, "msg-1", payload).await;

    // First run
    let mut tx = pool.begin().await.unwrap();
    migrate_delete_temp_paths(&mut tx, root_dir.path())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let row1 = sqlx::query("SELECT kind FROM outbox_messages WHERE id = 'msg-1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row1.get::<String, _>("kind"), "delete_workspace_file");

    // Second run
    let mut tx = pool.begin().await.unwrap();
    migrate_delete_temp_paths(&mut tx, root_dir.path())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let row2 = sqlx::query("SELECT kind FROM outbox_messages WHERE id = 'msg-1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row2.get::<String, _>("kind"), "delete_workspace_file");
}
