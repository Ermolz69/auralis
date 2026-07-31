#![allow(clippy::unwrap_used, clippy::expect_used)]
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
            id TEXT PRIMARY KEY NOT NULL,
            kind TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            attempts INTEGER NOT NULL DEFAULT 0,
            next_attempt_at TEXT NOT NULL,
            locked_at TEXT,
            locked_by TEXT,
            last_error TEXT,
            deduplication_key TEXT UNIQUE,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            aggregate_type TEXT,
            aggregate_id TEXT
        );
        CREATE INDEX idx_outbox_aggregate_status ON outbox_messages(aggregate_type, aggregate_id, status);

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
    let migrated: domain::outbox::OutboxPayload = serde_json::from_str(&payload_str).unwrap();

    match migrated {
        domain::outbox::OutboxPayload::DeleteWorkspaceFile { workspace_key } => {
            assert_eq!(workspace_key.as_str(), "tmp/some_file.txt");
        }
        _ => panic!("Expected DeleteWorkspaceFile payload"),
    }
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
    let migrated: domain::outbox::OutboxPayload = serde_json::from_str(&payload_str).unwrap();
    match migrated {
        domain::outbox::OutboxPayload::DeleteWorkspaceFile { workspace_key } => {
            assert_eq!(workspace_key.as_str(), "tmp/missing_dir/file.txt");
        }
        _ => panic!("Expected DeleteWorkspaceFile payload"),
    }
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

    let payload = json!({
        "type": "delete_temp_path",
        "path": "tmp/../outside.txt"
    });
    insert_legacy_message(&pool, "msg-1", payload).await;

    let payload2 = json!({
        "type": "delete_temp_path",
        "path": "tmp/./file.txt"
    });
    insert_legacy_message(&pool, "msg-2", payload2).await;

    let mut tx = pool.begin().await.unwrap();
    migrate_delete_temp_paths(&mut tx, root_dir.path())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    for id in ["msg-1", "msg-2"] {
        let row = sqlx::query("SELECT status, last_error FROM outbox_messages WHERE id = ?")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(row.get::<String, _>("status"), "dead");
        assert_eq!(
            row.get::<String, _>("last_error"),
            "Legacy DeleteTempPath contains invalid components"
        );
    }
}

#[tokio::test]
async fn legacy_relative_path_with_invalid_chars_to_dead() {
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

async fn setup_backfill_db() -> sqlx::SqlitePool {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn test_backfill_artifacts_clears_json_and_migrates_data() {
    let pool = setup_backfill_db().await;

    // Insert legacy project
    let project_id = uuid::Uuid::new_v4().to_string();
    let artifact_id = uuid::Uuid::new_v4().to_string();

    let legacy_json = json!([{
        "id": artifact_id,
        "kind": "SourceVideo",
        "location": { "LocalPath": "/tmp/test.mp4" },
        "size_bytes": 1234,
        "created_at": "2026-01-01T00:00:00Z"
    }]);

    sqlx::query(
        "INSERT INTO projects (id, title, status, source_json, metadata_json, source_language, target_language, transcript_json, active_job_id, last_terminal_job_id, artifacts_json, created_at, updated_at) VALUES (?, 'Legacy', 'Draft', NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"
    )
    .bind(&project_id)
    .bind(legacy_json.to_string())
    .execute(&pool)
    .await
    .unwrap();

    // Run backfill
    crate::sqlite::migrations_runtime::backfill_artifacts::run(&pool)
        .await
        .unwrap();

    // Verify artifacts_json is NOT gone
    let has_column: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('projects') WHERE name = 'artifacts_json'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(has_column, 1);

    // Verify JSON is cleared to '[]'
    let row = sqlx::query("SELECT artifacts_json FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let remaining_json: String = row.get("artifacts_json");
    assert_eq!(remaining_json, "[]");

    // Verify artifact is migrated
    let artifact_row = sqlx::query("SELECT * FROM artifacts WHERE id = ?")
        .bind(&artifact_id)
        .fetch_one(&pool)
        .await
        .unwrap();

    let kind: String = artifact_row.get("kind");
    let loc_kind: String = artifact_row.get("location_kind");
    let loc_val: String = artifact_row.get("location_value");

    assert_eq!(kind, "SourceVideo");
    assert_eq!(loc_kind, "LocalPath");
    assert_eq!(loc_val, "/tmp/test.mp4");
}

#[tokio::test]
async fn test_backfill_artifacts_fresh_db_keeps_column() {
    let pool = setup_backfill_db().await;

    // Run backfill (no legacy projects inserted)
    let report = crate::sqlite::migrations_runtime::backfill_artifacts::run(&pool)
        .await
        .unwrap();

    assert_eq!(report.projects_scanned, 0);

    // Verify artifacts_json is still in the schema
    let has_column: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('projects') WHERE name = 'artifacts_json'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(has_column, 1);
}

#[tokio::test]
async fn test_backfill_artifacts_leaves_json_on_failure() {
    let pool = setup_backfill_db().await;

    let project_id = uuid::Uuid::new_v4().to_string();
    let artifact_id = uuid::Uuid::new_v4().to_string();

    // Missing required location field will cause parsing error
    let legacy_json = json!([{
        "id": artifact_id,
        "kind": "SourceVideo"
    }]);

    sqlx::query(
        "INSERT INTO projects (id, title, status, source_json, metadata_json, source_language, target_language, transcript_json, active_job_id, last_terminal_job_id, artifacts_json, created_at, updated_at) VALUES (?, 'Legacy', 'Draft', NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"
    )
    .bind(&project_id)
    .bind(legacy_json.to_string())
    .execute(&pool)
    .await
    .unwrap();

    // Run backfill (should fail due to missing fields)
    let err = crate::sqlite::migrations_runtime::backfill_artifacts::run(&pool)
        .await
        .unwrap_err();

    assert!(err.to_string().contains("Backfill failed for 1 projects"));

    // Verify artifacts_json is NOT cleared
    let row = sqlx::query("SELECT artifacts_json FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let remaining_json: String = row.get("artifacts_json");
    assert_eq!(remaining_json, legacy_json.to_string());
}

#[tokio::test]
async fn test_equivalent_artifact_exists() {
    let pool = setup_backfill_db().await;

    let project_id = uuid::Uuid::new_v4().to_string();
    let artifact_id = uuid::Uuid::new_v4().to_string();

    let legacy_json = json!([{
        "id": artifact_id,
        "kind": "SourceVideo",
        "location": { "LocalPath": "/tmp/test.mp4" },
        "size_bytes": 1234,
        "created_at": "2026-01-01T00:00:00Z"
    }]);

    sqlx::query(
        "INSERT INTO projects (id, title, status, source_json, metadata_json, source_language, target_language, transcript_json, active_job_id, last_terminal_job_id, artifacts_json, created_at, updated_at) VALUES (?, 'Legacy', 'Draft', NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"
    )
    .bind(&project_id)
    .bind(legacy_json.to_string())
    .execute(&pool)
    .await
    .unwrap();

    // Manually insert the SAME artifact into the artifacts table
    sqlx::query(
        "INSERT INTO artifacts (id, project_id, kind, location_kind, location_value, size_bytes, state, created_at, updated_at) VALUES (?, ?, 'SourceVideo', 'LocalPath', '/tmp/test.mp4', 1234, 'ready', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"
    )
    .bind(&artifact_id)
    .bind(&project_id)
    .execute(&pool)
    .await
    .unwrap();

    // Run backfill
    let report = crate::sqlite::migrations_runtime::backfill_artifacts::run(&pool)
        .await
        .unwrap();

    assert_eq!(report.artifacts_already_present, 1);
    assert_eq!(report.failed_projects, 0);

    // Verify JSON is cleared
    let row = sqlx::query("SELECT artifacts_json FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let remaining_json: String = row.get("artifacts_json");
    assert_eq!(remaining_json, "[]");
}

#[tokio::test]
async fn test_same_id_different_data_fails() {
    let pool = setup_backfill_db().await;

    let project_id = uuid::Uuid::new_v4().to_string();
    let artifact_id = uuid::Uuid::new_v4().to_string();

    let legacy_json = json!([{
        "id": artifact_id,
        "kind": "SourceVideo",
        "location": { "LocalPath": "/tmp/test.mp4" },
        "size_bytes": 1234
    }]);

    sqlx::query(
        "INSERT INTO projects (id, title, status, source_json, metadata_json, source_language, target_language, transcript_json, active_job_id, last_terminal_job_id, artifacts_json, created_at, updated_at) VALUES (?, 'Legacy', 'Draft', NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"
    )
    .bind(&project_id)
    .bind(legacy_json.to_string())
    .execute(&pool)
    .await
    .unwrap();

    // Manually insert the DIFFERENT artifact with the SAME id
    sqlx::query(
        "INSERT INTO artifacts (id, project_id, kind, location_kind, location_value, size_bytes, state, created_at, updated_at) VALUES (?, ?, 'SourceVideo', 'LocalPath', '/tmp/different.mp4', 1234, 'ready', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"
    )
    .bind(&artifact_id)
    .bind(&project_id)
    .execute(&pool)
    .await
    .unwrap();

    // Run backfill
    let err = crate::sqlite::migrations_runtime::backfill_artifacts::run(&pool)
        .await
        .unwrap_err();

    assert!(err.to_string().contains("Backfill failed for 1 projects"));

    // Verify artifacts_json remains unchanged
    let row = sqlx::query("SELECT artifacts_json FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let remaining_json: String = row.get("artifacts_json");
    assert_eq!(remaining_json, legacy_json.to_string());
}

#[tokio::test]
async fn test_same_location_different_id_fails() {
    let pool = setup_backfill_db().await;

    let project_id = uuid::Uuid::new_v4().to_string();
    let artifact_id = uuid::Uuid::new_v4().to_string();

    let legacy_json = json!([{
        "id": artifact_id,
        "kind": "SourceVideo",
        "location": { "LocalPath": "/tmp/conflict.mp4" }
    }]);

    sqlx::query(
        "INSERT INTO projects (id, title, status, source_json, metadata_json, source_language, target_language, transcript_json, active_job_id, last_terminal_job_id, artifacts_json, created_at, updated_at) VALUES (?, 'Legacy', 'Draft', NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"
    )
    .bind(&project_id)
    .bind(legacy_json.to_string())
    .execute(&pool)
    .await
    .unwrap();

    // Insert an artifact with a DIFFERENT id but the SAME location
    let other_artifact_id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO artifacts (id, project_id, kind, location_kind, location_value, size_bytes, state, created_at, updated_at) VALUES (?, ?, 'SourceVideo', 'LocalPath', '/tmp/conflict.mp4', 1234, 'ready', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"
    )
    .bind(&other_artifact_id)
    .bind(&project_id)
    .execute(&pool)
    .await
    .unwrap();

    // Run backfill
    let err = crate::sqlite::migrations_runtime::backfill_artifacts::run(&pool)
        .await
        .unwrap_err();

    assert!(err.to_string().contains("Backfill failed for 1 projects"));

    // Verify artifacts_json remains unchanged
    let row = sqlx::query("SELECT artifacts_json FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let remaining_json: String = row.get("artifacts_json");
    assert_eq!(remaining_json, legacy_json.to_string());
}

#[tokio::test]
async fn test_size_bytes_exceeds_i64_max_fails() {
    let pool = setup_backfill_db().await;

    let project_id = uuid::Uuid::new_v4().to_string();
    let artifact_id = uuid::Uuid::new_v4().to_string();

    let legacy_json = json!([{
        "id": artifact_id,
        "kind": "SourceVideo",
        "location": { "LocalPath": "/tmp/test.mp4" },
        "size_bytes": u64::MAX
    }]);

    sqlx::query(
        "INSERT INTO projects (id, title, status, source_json, metadata_json, source_language, target_language, transcript_json, active_job_id, last_terminal_job_id, artifacts_json, created_at, updated_at) VALUES (?, 'Legacy', 'Draft', NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"
    )
    .bind(&project_id)
    .bind(legacy_json.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let err = crate::sqlite::migrations_runtime::backfill_artifacts::run(&pool)
        .await
        .unwrap_err();

    assert!(err.to_string().contains("Backfill failed for 1 projects"));

    // Verify artifacts_json remains unchanged
    let row = sqlx::query("SELECT artifacts_json FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let remaining_json: String = row.get("artifacts_json");
    assert_eq!(remaining_json, legacy_json.to_string());
}

#[tokio::test]
async fn test_partial_insert_rollback() {
    let pool = setup_backfill_db().await;

    let project_id = uuid::Uuid::new_v4().to_string();
    let first_art_id = uuid::Uuid::new_v4().to_string();
    let second_art_id = uuid::Uuid::new_v4().to_string();

    // The first is valid. The second is invalid (missing location).
    let legacy_json = json!([
        {
            "id": first_art_id,
            "kind": "SourceVideo",
            "location": { "LocalPath": "/tmp/valid.mp4" }
        },
        {
            "id": second_art_id,
            "kind": "SourceVideo"
        }
    ]);

    sqlx::query(
        "INSERT INTO projects (id, title, status, source_json, metadata_json, source_language, target_language, transcript_json, active_job_id, last_terminal_job_id, artifacts_json, created_at, updated_at) VALUES (?, 'Legacy', 'Draft', NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"
    )
    .bind(&project_id)
    .bind(legacy_json.to_string())
    .execute(&pool)
    .await
    .unwrap();

    // Run backfill
    let err = crate::sqlite::migrations_runtime::backfill_artifacts::run(&pool)
        .await
        .unwrap_err();

    assert!(err.to_string().contains("Backfill failed for 1 projects"));

    // Verify that the first artifact is NOT in the database (rolled back)
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM artifacts WHERE id = ?)")
        .bind(&first_art_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!exists);

    // Verify JSON remains original
    let row = sqlx::query("SELECT artifacts_json FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let remaining_json: String = row.get("artifacts_json");
    assert_eq!(remaining_json, legacy_json.to_string());
}

#[tokio::test]
async fn test_cas_rows_affected_zero_rollback() {
    let pool = setup_backfill_db().await;

    let project_id = uuid::Uuid::new_v4().to_string();
    let artifact_id = uuid::Uuid::new_v4().to_string();

    let legacy_json = json!([{
        "id": artifact_id,
        "kind": "SourceVideo",
        "location": { "LocalPath": "/tmp/test.mp4" }
    }]);

    sqlx::query(
        "INSERT INTO projects (id, title, status, source_json, metadata_json, source_language, target_language, transcript_json, active_job_id, last_terminal_job_id, artifacts_json, created_at, updated_at) VALUES (?, 'Legacy', 'Draft', NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"
    )
    .bind(&project_id)
    .bind(legacy_json.to_string())
    .execute(&pool)
    .await
    .unwrap();

    // Create trigger to ignore update, simulating CAS mismatch (rows_affected == 0)
    sqlx::query(
        "CREATE TRIGGER test_cas_trigger BEFORE UPDATE OF artifacts_json ON projects BEGIN SELECT RAISE(IGNORE); END;"
    )
    .execute(&pool)
    .await
    .unwrap();

    let err = crate::sqlite::migrations_runtime::backfill_artifacts::run(&pool)
        .await
        .unwrap_err();

    assert!(err.to_string().contains("Backfill failed for 1 projects"));

    // Verify first artifact rolled back
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM artifacts WHERE id = ?)")
        .bind(&artifact_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!exists);

    // Verify JSON remains original
    let row = sqlx::query("SELECT artifacts_json FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let remaining_json: String = row.get("artifacts_json");
    assert_eq!(remaining_json, legacy_json.to_string());
}

#[tokio::test]
async fn test_sql_error_on_cas_clear_rollback() {
    let pool = setup_backfill_db().await;

    let project_id = uuid::Uuid::new_v4().to_string();
    let artifact_id = uuid::Uuid::new_v4().to_string();

    let legacy_json = json!([{
        "id": artifact_id,
        "kind": "SourceVideo",
        "location": { "LocalPath": "/tmp/test.mp4" }
    }]);

    sqlx::query(
        "INSERT INTO projects (id, title, status, source_json, metadata_json, source_language, target_language, transcript_json, active_job_id, last_terminal_job_id, artifacts_json, created_at, updated_at) VALUES (?, 'Legacy', 'Draft', NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"
    )
    .bind(&project_id)
    .bind(legacy_json.to_string())
    .execute(&pool)
    .await
    .unwrap();

    // Create trigger to abort update with SQL error
    sqlx::query(
        "CREATE TRIGGER test_cas_err_trigger BEFORE UPDATE OF artifacts_json ON projects BEGIN SELECT RAISE(ABORT, 'Test CAS SQL Error'); END;"
    )
    .execute(&pool)
    .await
    .unwrap();

    let err = crate::sqlite::migrations_runtime::backfill_artifacts::run(&pool)
        .await
        .unwrap_err();

    assert!(err.to_string().contains("Backfill failed for 1 projects"));

    // Verify artifact rolled back
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM artifacts WHERE id = ?)")
        .bind(&artifact_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!exists);

    // Verify JSON remains original
    let row = sqlx::query("SELECT artifacts_json FROM projects WHERE id = ?")
        .bind(&project_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let remaining_json: String = row.get("artifacts_json");
    assert_eq!(remaining_json, legacy_json.to_string());
}

#[tokio::test]
async fn test_rerun_backfill_is_noop() {
    let pool = setup_backfill_db().await;

    let project_id = uuid::Uuid::new_v4().to_string();
    let artifact_id = uuid::Uuid::new_v4().to_string();

    let legacy_json = json!([{
        "id": artifact_id,
        "kind": "SourceVideo",
        "location": { "LocalPath": "/tmp/test.mp4" }
    }]);

    sqlx::query(
        "INSERT INTO projects (id, title, status, source_json, metadata_json, source_language, target_language, transcript_json, active_job_id, last_terminal_job_id, artifacts_json, created_at, updated_at) VALUES (?, 'Legacy', 'Draft', NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"
    )
    .bind(&project_id)
    .bind(legacy_json.to_string())
    .execute(&pool)
    .await
    .unwrap();

    // First run (success)
    let report1 = crate::sqlite::migrations_runtime::backfill_artifacts::run(&pool)
        .await
        .unwrap();
    assert_eq!(report1.projects_migrated, 1);
    assert_eq!(report1.artifacts_migrated, 1);

    // Second run (no-op since artifacts_json is already '[]')
    let report2 = crate::sqlite::migrations_runtime::backfill_artifacts::run(&pool)
        .await
        .unwrap();
    assert_eq!(report2.projects_scanned, 0);
    assert_eq!(report2.projects_migrated, 0);
    assert_eq!(report2.artifacts_migrated, 0);
}

#[tokio::test]
async fn test_independent_projects() {
    let pool = setup_backfill_db().await;

    let proj_a_id = uuid::Uuid::new_v4().to_string();
    let proj_b_id = uuid::Uuid::new_v4().to_string();
    let art_a_id = uuid::Uuid::new_v4().to_string();
    let art_b_id = uuid::Uuid::new_v4().to_string();

    let json_a = json!([{
        "id": art_a_id,
        "kind": "SourceVideo",
        "location": { "LocalPath": "/tmp/a.mp4" }
    }]);

    // Project B has malformed JSON array element (missing location)
    let json_b = json!([{
        "id": art_b_id,
        "kind": "SourceVideo"
    }]);

    sqlx::query(
        "INSERT INTO projects (id, title, status, source_json, metadata_json, source_language, target_language, transcript_json, active_job_id, last_terminal_job_id, artifacts_json, created_at, updated_at) VALUES (?, 'ProjA', 'Draft', NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"
    )
    .bind(&proj_a_id)
    .bind(json_a.to_string())
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO projects (id, title, status, source_json, metadata_json, source_language, target_language, transcript_json, active_job_id, last_terminal_job_id, artifacts_json, created_at, updated_at) VALUES (?, 'ProjB', 'Draft', NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"
    )
    .bind(&proj_b_id)
    .bind(json_b.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let err = crate::sqlite::migrations_runtime::backfill_artifacts::run(&pool)
        .await
        .unwrap_err();

    assert!(err.to_string().contains("Backfill failed for 1 projects"));

    // Verify Project A is migrated
    let exists_a: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM artifacts WHERE id = ?)")
        .bind(&art_a_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(exists_a);

    let row_a = sqlx::query("SELECT artifacts_json FROM projects WHERE id = ?")
        .bind(&proj_a_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row_a.get::<String, _>("artifacts_json"), "[]");

    // Verify Project B is rolled back and unchanged
    let exists_b: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM artifacts WHERE id = ?)")
        .bind(&art_b_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!exists_b);

    let row_b = sqlx::query("SELECT artifacts_json FROM projects WHERE id = ?")
        .bind(&proj_b_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row_b.get::<String, _>("artifacts_json"), json_b.to_string());
}

#[tokio::test]
async fn test_missing_column_and_marker_variations() {
    let pool = setup_backfill_db().await;

    // Verify that running with marker inserted but column existing STILL checks column (does not skip)
    sqlx::query("INSERT INTO runtime_migrations (id, applied_at) VALUES ('artifacts_json_dropped_v1', datetime('now'))")
        .execute(&pool)
        .await
        .unwrap();

    let project_id = uuid::Uuid::new_v4().to_string();
    let artifact_id = uuid::Uuid::new_v4().to_string();
    let legacy_json = json!([{
        "id": artifact_id,
        "kind": "SourceVideo",
        "location": { "LocalPath": "/tmp/test.mp4" }
    }]);

    sqlx::query(
        "INSERT INTO projects (id, title, status, source_json, metadata_json, source_language, target_language, transcript_json, active_job_id, last_terminal_job_id, artifacts_json, created_at, updated_at) VALUES (?, 'Legacy', 'Draft', NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')"
    )
    .bind(&project_id)
    .bind(legacy_json.to_string())
    .execute(&pool)
    .await
    .unwrap();

    // Run backfill - it should STILL migrate despite the marker
    let report = crate::sqlite::migrations_runtime::backfill_artifacts::run(&pool)
        .await
        .unwrap();
    assert_eq!(report.projects_migrated, 1);
    assert_eq!(report.artifacts_migrated, 1);

    // Now recreate projects table without artifacts_json column to simulate dropped column database
    sqlx::query("CREATE TABLE projects_temp AS SELECT id, title, status, source_json, metadata_json, source_language, target_language, transcript_json, active_job_id, last_terminal_job_id, created_at, updated_at FROM projects")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DROP TABLE projects")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE projects_temp RENAME TO projects")
        .execute(&pool)
        .await
        .unwrap();

    // Run backfill - should return Ok report immediately
    let report_missing = crate::sqlite::migrations_runtime::backfill_artifacts::run(&pool)
        .await
        .unwrap();
    assert_eq!(report_missing.projects_scanned, 0);
}
