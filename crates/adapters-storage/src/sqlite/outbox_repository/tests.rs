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
async fn outbox_prune_terminal_rows() {
    let pool = setup_db().await;
    let repo = SqliteOutboxRepository::new(pool.clone());

    let old_time = domain::chrono::Utc::now() - domain::chrono::Duration::days(10);
    let recent_time = domain::chrono::Utc::now() - domain::chrono::Duration::days(1);

    let old_time_str = old_time.to_rfc3339_opts(domain::chrono::SecondsFormat::Secs, true);
    let recent_time_str = recent_time.to_rfc3339_opts(domain::chrono::SecondsFormat::Secs, true);

    // 2 done (old), 1 done (recent), 2 dead (old), 1 dead (recent), 1 pending (old)
    sqlx::query(
        r#"
        INSERT INTO outbox_messages (
            id, kind, payload_json, status, attempts, next_attempt_at, created_at, updated_at
        ) VALUES 
        ('done1', 'x', '{}', 'done', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', ?),
        ('done2', 'x', '{}', 'done', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', ?),
        ('done3', 'x', '{}', 'done', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', ?),
        ('dead1', 'x', '{}', 'dead', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', ?),
        ('dead2', 'x', '{}', 'dead', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', ?),
        ('dead3', 'x', '{}', 'dead', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', ?),
        ('pend1', 'x', '{}', 'pending', 0, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', ?)
        "#,
    )
    .bind(&old_time_str)
    .bind(&old_time_str)
    .bind(&recent_time_str)
    .bind(&old_time_str)
    .bind(&old_time_str)
    .bind(&recent_time_str)
    .bind(&old_time_str)
    .execute(&pool)
    .await
    .unwrap();

    let done_cutoff = domain::chrono::Utc::now() - domain::chrono::Duration::days(5);
    let dead_cutoff = domain::chrono::Utc::now() - domain::chrono::Duration::days(5);

    // limit = 1 per status
    let report = repo
        .prune_terminal_rows(done_cutoff, dead_cutoff, 1)
        .await
        .unwrap();
    assert_eq!(report.done_deleted, 1);
    assert_eq!(report.dead_deleted, 1);

    // limit = 10 (should delete remaining 1 of each old)
    let report2 = repo
        .prune_terminal_rows(done_cutoff, dead_cutoff, 10)
        .await
        .unwrap();
    assert_eq!(report2.done_deleted, 1);
    assert_eq!(report2.dead_deleted, 1);

    use sqlx::Row;
    let count: i64 = sqlx::query("SELECT COUNT(*) as c FROM outbox_messages")
        .fetch_one(&pool)
        .await
        .unwrap()
        .get("c");

    // 3 remain: done3, dead3, pend1
    assert_eq!(count, 3);
}
