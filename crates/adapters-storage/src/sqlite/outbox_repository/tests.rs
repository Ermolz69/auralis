#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::SqliteOutboxRepository;
use domain::outbox::{OutboxMessage, OutboxMessageId, OutboxPayload};
use domain::project::ProjectId;
use ports::repository::OutboxRepository;
use sqlx::SqlitePool;

async fn setup_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
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

fn sample_payload() -> OutboxPayload {
    OutboxPayload::DeleteProjectArtifactDir {
        project_id: ProjectId::new(),
    }
}

#[tokio::test]
async fn outbox_insert_and_fetch_pending() {
    let pool = setup_db().await;
    let repo = SqliteOutboxRepository::new(pool.clone());

    let msg = OutboxMessage::new(sample_payload());
    let payload_json = serde_json::to_string(&msg.payload).unwrap();

    sqlx::query(
        r#"
        INSERT INTO outbox_messages (
            id, kind, payload_json, status, attempts, next_attempt_at,
            created_at, updated_at
        ) VALUES (?, 'delete_project_artifact_dir', ?, 'pending', 0, strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-1 minute'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
        "#
    )
    .bind(msg.id.to_string())
    .bind(payload_json)
    .execute(&pool)
    .await
    .unwrap();

    let pending = repo.fetch_pending(10).await.unwrap();
    assert_eq!(pending.messages.len(), 1);
    assert_eq!(pending.messages[0].id, msg.id);
}

#[tokio::test]
async fn outbox_claim_changes_status() {
    let pool = setup_db().await;
    let repo = SqliteOutboxRepository::new(pool.clone());

    let msg = OutboxMessage::new(sample_payload());
    let payload_json = serde_json::to_string(&msg.payload).unwrap();

    sqlx::query(
        r#"
        INSERT INTO outbox_messages (
            id, kind, payload_json, status, attempts, next_attempt_at,
            created_at, updated_at
        ) VALUES (?, 'delete_project_artifact_dir', ?, 'pending', 0, strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-1 minute'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
        "#
    )
    .bind(msg.id.to_string())
    .bind(payload_json)
    .execute(&pool)
    .await
    .unwrap();

    let claimed = repo.mark_processing(&msg.id, "worker-1").await.unwrap();
    assert!(claimed);

    // Try to claim again
    let claimed_again = repo.mark_processing(&msg.id, "worker-2").await.unwrap();
    assert!(!claimed_again);

    use sqlx::Row;
    let row = sqlx::query("SELECT status, locked_by FROM outbox_messages WHERE id = ?")
        .bind(msg.id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.get::<String, _>("status"), "processing");
    assert_eq!(
        row.get::<Option<String>, _>("locked_by").as_deref(),
        Some("worker-1")
    );
}

#[tokio::test]
async fn outbox_mark_done() {
    let pool = setup_db().await;
    let repo = SqliteOutboxRepository::new(pool.clone());

    let msg = OutboxMessage::new(sample_payload());
    let payload_json = serde_json::to_string(&msg.payload).unwrap();

    sqlx::query(
        r#"
        INSERT INTO outbox_messages (
            id, kind, payload_json, status, attempts, next_attempt_at,
            locked_at, locked_by, created_at, updated_at
        ) VALUES (?, 'delete_project_artifact_dir', ?, 'processing', 0, strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-1 minute'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), 'worker-1', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
        "#
    )
    .bind(msg.id.to_string())
    .bind(payload_json)
    .execute(&pool)
    .await
    .unwrap();

    repo.mark_done(&msg.id).await.unwrap();

    use sqlx::Row;
    let row = sqlx::query("SELECT status, locked_by, locked_at FROM outbox_messages WHERE id = ?")
        .bind(msg.id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.get::<String, _>("status"), "done");
    assert!(row.get::<Option<String>, _>("locked_by").is_none());
    assert!(row.get::<Option<String>, _>("locked_at").is_none());
}

#[tokio::test]
async fn outbox_mark_failed_sets_backoff() {
    let pool = setup_db().await;
    let repo = SqliteOutboxRepository::new(pool.clone());

    let msg = OutboxMessage::new(sample_payload());
    let payload_json = serde_json::to_string(&msg.payload).unwrap();

    sqlx::query(
        r#"
        INSERT INTO outbox_messages (
            id, kind, payload_json, status, attempts, next_attempt_at,
            locked_at, locked_by, created_at, updated_at
        ) VALUES (?, 'delete_project_artifact_dir', ?, 'processing', 0, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), 'worker-1', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
        "#
    )
    .bind(msg.id.to_string())
    .bind(payload_json)
    .execute(&pool)
    .await
    .unwrap();

    repo.mark_failed(&msg.id, "Network timeout").await.unwrap();

    use sqlx::Row;
    let row = sqlx::query(
        "SELECT status, attempts, last_error, locked_by FROM outbox_messages WHERE id = ?",
    )
    .bind(msg.id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.get::<String, _>("status"), "failed");
    assert_eq!(row.get::<i64, _>("attempts"), 1);
    assert_eq!(
        row.get::<Option<String>, _>("last_error").as_deref(),
        Some("Network timeout")
    );
    assert!(row.get::<Option<String>, _>("locked_by").is_none());
}

#[tokio::test]
async fn outbox_dead_after_max_attempts() {
    let pool = setup_db().await;
    let repo = SqliteOutboxRepository::new(pool.clone());

    let msg = OutboxMessage::new(sample_payload());
    let payload_json = serde_json::to_string(&msg.payload).unwrap();

    sqlx::query(
        r#"
        INSERT INTO outbox_messages (
            id, kind, payload_json, status, attempts, next_attempt_at,
            created_at, updated_at
        ) VALUES (?, 'delete_project_artifact_dir', ?, 'processing', 4, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
        "#
    )
    .bind(msg.id.to_string())
    .bind(payload_json)
    .execute(&pool)
    .await
    .unwrap();

    repo.mark_failed(&msg.id, "Still failing").await.unwrap();

    use sqlx::Row;
    let row = sqlx::query("SELECT status, attempts FROM outbox_messages WHERE id = ?")
        .bind(msg.id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.get::<String, _>("status"), "dead");
    assert_eq!(row.get::<i64, _>("attempts"), 5);
}

#[tokio::test]
async fn corrupted_payload_becomes_error_or_dead() {
    let pool = setup_db().await;
    let repo = SqliteOutboxRepository::new(pool.clone());

    let msg1 = OutboxMessage::new(sample_payload());
    let payload_json1 = serde_json::to_string(&msg1.payload).unwrap();

    let msg3 = OutboxMessage::new(sample_payload());
    let payload_json3 = serde_json::to_string(&msg3.payload).unwrap();

    let corrupted_id = OutboxMessageId::new().to_string();

    // Insert 1 valid, 1 corrupted, 1 valid
    sqlx::query(
        r#"
        INSERT INTO outbox_messages (
            id, kind, payload_json, status, attempts, next_attempt_at,
            created_at, updated_at
        ) VALUES 
        (?, 'delete_project_artifact_dir', ?, 'pending', 0, strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-1 minute'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-2 minute'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
        (?, 'delete_project_artifact_dir', 'invalid json', 'pending', 0, strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-1 minute'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-1 minute'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
        (?, 'delete_project_artifact_dir', ?, 'pending', 0, strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-1 minute'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
        "#
    )
    .bind(msg1.id.to_string())
    .bind(payload_json1)
    .bind(&corrupted_id)
    .bind(msg3.id.to_string())
    .bind(payload_json3)
    .execute(&pool)
    .await
    .unwrap();

    // fetch_pending should skip the corrupted one, mark it dead, and return the 2 valid ones
    let pending = repo.fetch_pending(10).await.unwrap();
    assert_eq!(pending.messages.len(), 2);
    assert_eq!(pending.messages[0].id, msg1.id);
    assert_eq!(pending.messages[1].id, msg3.id);

    use sqlx::Row;
    let row = sqlx::query("SELECT status, last_error FROM outbox_messages WHERE id = ?")
        .bind(&corrupted_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.get::<String, _>("status"), "dead");
    assert!(
        row.get::<String, _>("last_error")
            .contains("Corrupted outbox payload")
    );
}

#[tokio::test]
async fn invalid_outbox_id_does_not_block_batch() {
    let pool = setup_db().await;
    let repo = SqliteOutboxRepository::new(pool.clone());

    let msg1 = OutboxMessage::new(sample_payload());
    let payload_json1 = serde_json::to_string(&msg1.payload).unwrap();

    let invalid_id = "not-a-uuid";

    // Insert 1 invalid id, 1 valid
    sqlx::query(
        r#"
        INSERT INTO outbox_messages (
            id, kind, payload_json, status, attempts, next_attempt_at,
            created_at, updated_at
        ) VALUES 
        (?, 'delete_project_artifact_dir', '{}', 'pending', 0, strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-1 minute'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-1 minute'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
        (?, 'delete_project_artifact_dir', ?, 'pending', 0, strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-1 minute'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
        "#
    )
    .bind(invalid_id)
    .bind(msg1.id.to_string())
    .bind(payload_json1)
    .execute(&pool)
    .await
    .unwrap();

    let pending = repo.fetch_pending(10).await.unwrap();
    assert_eq!(pending.messages.len(), 1);
    assert_eq!(pending.messages[0].id, msg1.id);

    use sqlx::Row;
    let row = sqlx::query("SELECT status, last_error FROM outbox_messages WHERE id = ?")
        .bind(invalid_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.get::<String, _>("status"), "dead");
}

#[tokio::test]
async fn mark_done_fails_if_already_done_or_missing() {
    let pool = setup_db().await;
    let repo = SqliteOutboxRepository::new(pool.clone());

    let result = repo.mark_done(&OutboxMessageId::new()).await;
    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(err_str.contains("not found or already modified"));
}

#[tokio::test]
async fn mark_done_is_idempotent_on_done_and_dead() {
    let pool = setup_db().await;
    let repo = SqliteOutboxRepository::new(pool.clone());

    // 1. Test 'done' message
    let msg_done_id = OutboxMessageId::new();
    sqlx::query(
        "INSERT INTO outbox_messages (id, kind, payload_json, status, attempts, next_attempt_at, created_at, updated_at) VALUES (?, 'delete_project_artifact_dir', '{}', 'done', 0, 'now', 'now', 'now')"
    )
    .bind(msg_done_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let res = repo.mark_done(&msg_done_id).await;
    assert!(res.is_ok());

    // 2. Test 'dead' message
    let msg_dead_id = OutboxMessageId::new();
    sqlx::query(
        "INSERT INTO outbox_messages (id, kind, payload_json, status, attempts, next_attempt_at, created_at, updated_at) VALUES (?, 'delete_project_artifact_dir', '{}', 'dead', 0, 'now', 'now', 'now')"
    )
    .bind(msg_dead_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let res = repo.mark_done(&msg_dead_id).await;
    assert!(res.is_ok());

    // 3. Test 'pending' message fails
    let msg_pending_id = OutboxMessageId::new();
    sqlx::query(
        "INSERT INTO outbox_messages (id, kind, payload_json, status, attempts, next_attempt_at, created_at, updated_at) VALUES (?, 'delete_project_artifact_dir', '{}', 'pending', 0, 'now', 'now', 'now')"
    )
    .bind(msg_pending_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    let res = repo.mark_done(&msg_pending_id).await;
    assert!(res.is_err());
    let err_str = res.unwrap_err().to_string();
    assert!(err_str.contains("not found or already modified"));
}

#[tokio::test]
async fn mark_failed_fails_if_missing() {
    let pool = setup_db().await;
    let repo = SqliteOutboxRepository::new(pool.clone());

    let result = repo.mark_failed(&OutboxMessageId::new(), "error").await;
    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(err_str.contains("not found or already modified"));
}

#[tokio::test]
async fn stale_lock_reclaim() {
    let pool = setup_db().await;
    let repo = SqliteOutboxRepository::new(pool.clone());

    let msg = OutboxMessage::new(sample_payload());
    let payload_json = serde_json::to_string(&msg.payload).unwrap();

    sqlx::query(
        r#"
        INSERT INTO outbox_messages (
            id, kind, payload_json, status, attempts, next_attempt_at,
            locked_at, locked_by, created_at, updated_at
        ) VALUES (?, 'delete_project_artifact_dir', ?, 'processing', 0, strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-10 minutes'), 'worker-1', strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
        "#
    )
    .bind(msg.id.to_string())
    .bind(payload_json)
    .execute(&pool)
    .await
    .unwrap();

    // fetch_pending reclaims stale locks implicitly
    let pending = repo.fetch_pending(10).await.unwrap();
    assert_eq!(pending.messages.len(), 1);

    use sqlx::Row;
    let row = sqlx::query("SELECT status, attempts, last_error FROM outbox_messages WHERE id = ?")
        .bind(msg.id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.get::<String, _>("status"), "pending");
    assert_eq!(row.get::<i64, _>("attempts"), 1);
    assert_eq!(
        row.get::<String, _>("last_error"),
        "Timeout during processing (stale lock reclaimed)"
    );
}

#[tokio::test]
async fn mark_dead_failure_breaks_fetch_loop() {
    let pool = setup_db().await;
    let repo = SqliteOutboxRepository::new(pool.clone());

    let corrupted_id = OutboxMessageId::new().to_string();

    sqlx::query(
        r#"
        INSERT INTO outbox_messages (
            id, kind, payload_json, status, attempts, next_attempt_at,
            created_at, updated_at
        ) VALUES 
        (?, 'delete_project_artifact_dir', 'invalid json', 'pending', 0, strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-1 minute'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-1 minute'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
        "#
    )
    .bind(&corrupted_id)
    .execute(&pool)
    .await
    .unwrap();

    // Create a trigger that fails any update to status = 'dead'
    sqlx::query(
        r#"
        CREATE TRIGGER prevent_mark_dead BEFORE UPDATE ON outbox_messages
        FOR EACH ROW WHEN NEW.status = 'dead'
        BEGIN
            SELECT RAISE(ABORT, 'Simulated mark_dead_raw failure');
        END;
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let result = repo.fetch_pending(10).await.unwrap();
    assert_eq!(result.messages.len(), 0);
    assert_eq!(result.corrupted_isolated, 0);
    assert_eq!(result.isolation_errors, 1);
}

#[tokio::test]
async fn outbox_prune_terminal_rows_comprehensive() {
    let pool = setup_db().await;
    let repo = SqliteOutboxRepository::new(pool.clone());

    // 1. Test "strict < cutoff"
    let cutoff_str = "2026-07-17T20:00:00Z";
    let cutoff = chrono::DateTime::parse_from_rfc3339(cutoff_str)
        .unwrap()
        .with_timezone(&chrono::Utc);

    sqlx::query(
        r#"
        INSERT INTO outbox_messages (
            id, kind, payload_json, status, attempts, next_attempt_at, created_at, updated_at
        ) VALUES 
        ('done_exact', 'x', '{}', 'done', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', '2026-07-17T20:00:00Z'),
        ('done_older', 'x', '{}', 'done', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', '2026-07-17T19:59:59Z')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let report = repo.prune_terminal_rows(cutoff, cutoff, 10).await.unwrap();
    assert_eq!(report.done_deleted, 1);
    assert_eq!(report.dead_deleted, 0);

    use sqlx::Row;
    let count: i64 =
        sqlx::query("SELECT COUNT(*) as c FROM outbox_messages WHERE id = 'done_exact'")
            .fetch_one(&pool)
            .await
            .unwrap()
            .get("c");
    assert_eq!(
        count, 1,
        "done_exact should NOT be deleted (strict < cutoff)"
    );

    // Clean up
    sqlx::query("DELETE FROM outbox_messages")
        .execute(&pool)
        .await
        .unwrap();

    // 2. Test "separate done/dead cutoff"
    sqlx::query(
        r#"
        INSERT INTO outbox_messages (
            id, kind, payload_json, status, attempts, next_attempt_at, created_at, updated_at
        ) VALUES 
        ('done1', 'x', '{}', 'done', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', '2026-07-17T15:00:00Z'),
        ('dead1', 'x', '{}', 'dead', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', '2026-07-17T10:00:00Z')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let done_cutoff = chrono::DateTime::parse_from_rfc3339("2026-07-17T16:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let dead_cutoff = chrono::DateTime::parse_from_rfc3339("2026-07-17T09:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    let report = repo
        .prune_terminal_rows(done_cutoff, dead_cutoff, 10)
        .await
        .unwrap();
    assert_eq!(
        report.done_deleted, 1,
        "done1 should be deleted (15:00 < 16:00)"
    );
    assert_eq!(
        report.dead_deleted, 0,
        "dead1 should NOT be deleted (10:00 is not < 09:00)"
    );

    // Clean up
    sqlx::query("DELETE FROM outbox_messages")
        .execute(&pool)
        .await
        .unwrap();

    // 3. Test "per-status limit" and "deterministic order"
    sqlx::query(
        r#"
        INSERT INTO outbox_messages (
            id, kind, payload_json, status, attempts, next_attempt_at, created_at, updated_at
        ) VALUES 
        ('done_c', 'x', '{}', 'done', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', '2020-01-01T12:00:00Z'),
        ('done_a', 'x', '{}', 'done', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', '2020-01-01T12:00:00Z'),
        ('done_b', 'x', '{}', 'done', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', '2020-01-01T12:00:00Z'),
        ('dead_b', 'x', '{}', 'dead', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', '2020-01-01T12:00:00Z'),
        ('dead_a', 'x', '{}', 'dead', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', '2020-01-01T12:00:00Z')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let cutoff = chrono::DateTime::parse_from_rfc3339("2026-07-17T20:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    // limit = 1 per status
    let report = repo.prune_terminal_rows(cutoff, cutoff, 1).await.unwrap();
    assert_eq!(report.done_deleted, 1);
    assert_eq!(report.dead_deleted, 1);

    // Verify deterministic order: smallest ID has been deleted
    let remaining_done: Vec<String> = sqlx::query_as::<_, (String,)>(
        "SELECT id FROM outbox_messages WHERE status = 'done' ORDER BY id ASC",
    )
    .fetch_all(&pool)
    .await
    .unwrap()
    .into_iter()
    .map(|r| r.0)
    .collect();
    assert_eq!(
        remaining_done,
        vec!["done_b".to_string(), "done_c".to_string()]
    ); // done_a deleted!

    let remaining_dead: Vec<String> = sqlx::query_as::<_, (String,)>(
        "SELECT id FROM outbox_messages WHERE status = 'dead' ORDER BY id ASC",
    )
    .fetch_all(&pool)
    .await
    .unwrap()
    .into_iter()
    .map(|r| r.0)
    .collect();
    assert_eq!(remaining_dead, vec!["dead_b".to_string()]); // dead_a deleted!

    // Clean up
    sqlx::query("DELETE FROM outbox_messages")
        .execute(&pool)
        .await
        .unwrap();

    // 4. Test "pending/processing/failed не удаляются"
    sqlx::query(
        r#"
        INSERT INTO outbox_messages (
            id, kind, payload_json, status, attempts, next_attempt_at, created_at, updated_at
        ) VALUES 
        ('pend1', 'x', '{}', 'pending', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', '2020-01-01T12:00:00Z'),
        ('proc1', 'x', '{}', 'processing', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', '2020-01-01T12:00:00Z'),
        ('fail1', 'x', '{}', 'failed', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', '2020-01-01T12:00:00Z')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let report = repo.prune_terminal_rows(cutoff, cutoff, 10).await.unwrap();
    assert_eq!(report.done_deleted, 0);
    assert_eq!(report.dead_deleted, 0);

    let count: i64 = sqlx::query("SELECT COUNT(*) as c FROM outbox_messages")
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("c");
    assert_eq!(
        count, 3,
        "None of pending, processing, failed should be deleted"
    );

    // Clean up
    sqlx::query("DELETE FROM outbox_messages")
        .execute(&pool)
        .await
        .unwrap();

    // 5. Test "rollback при втором delete"
    sqlx::query(
        r#"
        INSERT INTO outbox_messages (
            id, kind, payload_json, status, attempts, next_attempt_at, created_at, updated_at
        ) VALUES 
        ('done1', 'x', '{}', 'done', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', '2020-01-01T12:00:00Z'),
        ('dead1', 'x', '{}', 'dead', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', '2020-01-01T12:00:00Z')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    // Create a trigger that fails delete ON dead rows
    sqlx::query(
        r#"
        CREATE TRIGGER prevent_delete_dead BEFORE DELETE ON outbox_messages
        FOR EACH ROW WHEN OLD.status = 'dead'
        BEGIN
            SELECT RAISE(ABORT, 'Simulated dead prune fail');
        END;
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let result = repo.prune_terminal_rows(cutoff, cutoff, 10).await;
    assert!(
        result.is_err(),
        "Prune must return error due to trigger on dead delete"
    );

    // Verify transaction rollback: both done1 and dead1 must still exist!
    let count: i64 = sqlx::query("SELECT COUNT(*) as c FROM outbox_messages")
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("c");
    assert_eq!(count, 2, "Both rows must remain due to rollback");

    // Clean up trigger
    sqlx::query("DROP TRIGGER prevent_delete_dead")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM outbox_messages")
        .execute(&pool)
        .await
        .unwrap();

    // 6. Test "terminal updates меняют updated_at"
    let msg1_id = OutboxMessageId::new();
    sqlx::query(
        r#"
        INSERT INTO outbox_messages (
            id, kind, payload_json, status, attempts, next_attempt_at, created_at, updated_at
        ) VALUES 
        (?, 'x', '{}', 'processing', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', '2020-01-01T12:00:00Z')
        "#,
    )
    .bind(msg1_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    repo.mark_done(&msg1_id).await.unwrap();

    let updated_at: String =
        sqlx::query_as::<_, (String,)>("SELECT updated_at FROM outbox_messages WHERE id = ?")
            .bind(msg1_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap()
            .0;
    assert_ne!(
        updated_at, "2020-01-01T12:00:00Z",
        "updated_at must change on status update to done"
    );

    sqlx::query("DELETE FROM outbox_messages")
        .execute(&pool)
        .await
        .unwrap();

    // 7. Test failed -> dead (attempts = 4)
    let msg2_id = OutboxMessageId::new();
    sqlx::query(
        r#"
        INSERT INTO outbox_messages (
            id, kind, payload_json, status, attempts, next_attempt_at, created_at, updated_at
        ) VALUES 
        (?, 'x', '{}', 'processing', 4, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', '2020-01-01T12:00:00Z')
        "#,
    )
    .bind(msg2_id.to_string())
    .execute(&pool)
    .await
    .unwrap();

    repo.mark_failed(&msg2_id, "error").await.unwrap();

    let (status, attempts, updated_at): (String, i64, String) =
        sqlx::query_as("SELECT status, attempts, updated_at FROM outbox_messages WHERE id = ?")
            .bind(msg2_id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "dead");
    assert_eq!(attempts, 5);
    assert_ne!(updated_at, "2020-01-01T12:00:00Z");

    sqlx::query("DELETE FROM outbox_messages")
        .execute(&pool)
        .await
        .unwrap();

    // 8. Test "stale-lock reclaim меняет updated_at" (with valid payload)
    let msg = OutboxMessage::new(OutboxPayload::DeleteProjectArtifactDir {
        project_id: ProjectId::new(),
    });
    let payload_json = serde_json::to_string(&msg.payload).unwrap();
    sqlx::query(
        r#"
        INSERT INTO outbox_messages (
            id, kind, payload_json, status, attempts, next_attempt_at, locked_at, locked_by, created_at, updated_at
        ) VALUES 
        (?, 'delete_project_artifact_dir', ?, 'processing', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', 'worker1', '2020-01-01T00:00:00Z', '2020-01-01T12:00:00Z')
        "#,
    )
    .bind(msg.id.to_string())
    .bind(payload_json)
    .execute(&pool)
    .await
    .unwrap();

    // Reclaim should trigger
    let pending = repo.fetch_pending(10).await.unwrap();
    assert_eq!(pending.messages.len(), 1);

    let (status, updated_at): (String, String) =
        sqlx::query_as("SELECT status, updated_at FROM outbox_messages WHERE id = ?")
            .bind(msg.id.to_string())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "pending");
    assert_ne!(updated_at, "2020-01-01T12:00:00Z");
}
