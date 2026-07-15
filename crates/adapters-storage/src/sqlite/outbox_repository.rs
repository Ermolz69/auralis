use async_trait::async_trait;
use sqlx::{Pool, Sqlite};

use domain::outbox::{OutboxMessage, OutboxMessageId};
use ports::error::PortError;
use ports::repository::OutboxRepository;

use super::outbox_mapper::row_to_outbox_message;
use super::outbox_row::OutboxRow;

#[derive(Clone)]
pub struct SqliteOutboxRepository {
    pool: Pool<Sqlite>,
}

impl SqliteOutboxRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OutboxRepository for SqliteOutboxRepository {
    async fn fetch_pending(&self, limit: usize) -> Result<Vec<OutboxMessage>, PortError> {
        // Reclaim stale locks: messages in 'processing' state for more than 5 minutes
        sqlx::query(
            r#"
            UPDATE outbox_messages
            SET status = 'pending',
                attempts = attempts + 1,
                last_error = 'Timeout during processing (stale lock reclaimed)',
                locked_at = NULL,
                locked_by = NULL,
                next_attempt_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
            WHERE status = 'processing' 
              AND locked_at <= strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-5 minutes')
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| PortError::Io {
            message: format!("Failed to reclaim stale outbox locks: {}", e),
        })?;

        let rows = sqlx::query_as::<_, OutboxRow>(
            r#"
            SELECT 
                id, kind, payload_json, status, attempts, 
                next_attempt_at, locked_at, locked_by, last_error, deduplication_key, 
                created_at, updated_at
            FROM outbox_messages
            WHERE status IN ('pending', 'failed') AND next_attempt_at <= strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
            ORDER BY created_at ASC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| PortError::Io {
            message: format!("Failed to fetch pending outbox messages: {}", e),
        })?;

        let mut messages = Vec::new();
        for row in rows {
            let message = row_to_outbox_message(row)?;
            messages.push(message);
        }

        Ok(messages)
    }

    async fn mark_processing(
        &self,
        id: &OutboxMessageId,
        locked_by: &str,
    ) -> Result<bool, PortError> {
        let id_str = id.to_string();
        let result = sqlx::query(
            r#"
            UPDATE outbox_messages
            SET status = 'processing',
                locked_by = ?,
                locked_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
            WHERE id = ? 
              AND status IN ('pending', 'failed') 
              AND next_attempt_at <= strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
            "#,
        )
        .bind(locked_by)
        .bind(id_str)
        .execute(&self.pool)
        .await
        .map_err(|e| PortError::Io {
            message: format!("Failed to mark outbox message {} as processing: {}", id, e),
        })?;

        Ok(result.rows_affected() > 0)
    }

    async fn mark_done(&self, id: &OutboxMessageId) -> Result<(), PortError> {
        let id_str = id.to_string();
        sqlx::query(
            r#"
            UPDATE outbox_messages
            SET status = 'done',
                locked_at = NULL,
                locked_by = NULL,
                updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
            WHERE id = ?
            "#,
        )
        .bind(id_str)
        .execute(&self.pool)
        .await
        .map_err(|e| PortError::Io {
            message: format!("Failed to mark outbox message {} as done: {}", id, e),
        })?;

        Ok(())
    }

    async fn mark_failed(&self, id: &OutboxMessageId, error: &str) -> Result<(), PortError> {
        let id_str = id.to_string();

        let row = sqlx::query_as::<_, OutboxRow>(
            r#"
            SELECT 
                id, kind, payload_json, status, attempts, 
                next_attempt_at, locked_at, locked_by, last_error, deduplication_key, 
                created_at, updated_at
            FROM outbox_messages
            WHERE id = ?
            "#,
        )
        .bind(&id_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| PortError::Io {
            message: format!("Failed to fetch message {}: {}", id, e),
        })?;

        if let Some(r) = row {
            let mut message = row_to_outbox_message(r)?;
            message.attempts += 1;
            message.last_error = Some(error.to_string());
            message.locked_at = None;
            message.locked_by = None;

            if message.attempts >= 5 {
                message.status = domain::outbox::OutboxMessageStatus::Dead;
            } else {
                message.status = domain::outbox::OutboxMessageStatus::Failed;
                // Exponential backoff
                let delay_seconds = 2_i64.pow(message.attempts as u32) * 60;
                message.next_attempt_at =
                    domain::chrono::Utc::now() + domain::chrono::Duration::seconds(delay_seconds);
            }

            let status_str = match message.status {
                domain::outbox::OutboxMessageStatus::Pending => "pending",
                domain::outbox::OutboxMessageStatus::Processing => "processing",
                domain::outbox::OutboxMessageStatus::Done => "done",
                domain::outbox::OutboxMessageStatus::Failed => "failed",
                domain::outbox::OutboxMessageStatus::Dead => "dead",
            };

            sqlx::query(
                r#"
                UPDATE outbox_messages
                SET status = ?,
                    attempts = ?,
                    next_attempt_at = ?,
                    locked_at = NULL,
                    locked_by = NULL,
                    last_error = ?,
                    updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                WHERE id = ?
                "#,
            )
            .bind(status_str)
            .bind(message.attempts as i64)
            .bind(message.next_attempt_at.to_rfc3339())
            .bind(message.last_error)
            .bind(id_str)
            .execute(&self.pool)
            .await
            .map_err(|e| PortError::Io {
                message: format!("Failed to update outbox message {}: {}", id, e),
            })?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::outbox::OutboxPayload;
    use domain::project::ProjectId;
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
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
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
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, msg.id);
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
        let row =
            sqlx::query("SELECT status, locked_by, locked_at FROM outbox_messages WHERE id = ?")
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

        let msg_id = OutboxMessageId::new();

        sqlx::query(
            r#"
            INSERT INTO outbox_messages (
                id, kind, payload_json, status, attempts, next_attempt_at,
                created_at, updated_at
            ) VALUES (?, 'delete_project_artifact_dir', 'invalid json', 'pending', 0, strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '-1 minute'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
            "#
        )
        .bind(msg_id.to_string())
        .execute(&pool)
        .await
        .unwrap();

        // fetch_pending should skip it or fail. Since it maps inside loop, it fails the whole fetch currently.
        // But for testing purposes, if it's corrupted, we verify fetch_pending returns error
        let result = repo.fetch_pending(10).await;
        assert!(result.is_err());
    }
}
