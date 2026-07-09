use async_trait::async_trait;
use sqlx::{Pool, Row, Sqlite};

use domain::outbox::{OutboxMessage, OutboxMessageId};
use ports::error::PortError;
use ports::repository::OutboxRepository;

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
        let rows = sqlx::query(
            r#"
            SELECT data
            FROM outbox_messages
            WHERE status = 'pending' AND next_attempt_at <= datetime('now')
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
            let data: String = row.try_get("data").map_err(|e| PortError::Io {
                message: format!("Failed to read data column: {}", e),
            })?;
            let message: OutboxMessage =
                serde_json::from_str(&data).map_err(|e| PortError::Io {
                    message: format!("Failed to deserialize outbox message: {}", e),
                })?;
            messages.push(message);
        }

        Ok(messages)
    }

    async fn mark_processing(
        &self,
        id: &OutboxMessageId,
        locked_by: &str,
    ) -> Result<(), PortError> {
        let id_str = id.to_string();
        sqlx::query(
            r#"
            UPDATE outbox_messages
            SET status = 'processing',
                locked_by = ?,
                locked_at = datetime('now'),
                updated_at = datetime('now')
            WHERE id = ? AND status = 'pending'
            "#,
        )
        .bind(locked_by)
        .bind(id_str)
        .execute(&self.pool)
        .await
        .map_err(|e| PortError::Io {
            message: format!("Failed to mark outbox message {} as processing: {}", id, e),
        })?;

        Ok(())
    }

    async fn mark_done(&self, id: &OutboxMessageId) -> Result<(), PortError> {
        let id_str = id.to_string();
        sqlx::query(
            r#"
            UPDATE outbox_messages
            SET status = 'done',
                updated_at = datetime('now')
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
        let row = sqlx::query(
            r#"
            SELECT data
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
            let data: String = r.try_get("data").unwrap();
            let mut message: OutboxMessage = serde_json::from_str(&data).unwrap();
            message.attempts += 1;
            message.last_error = Some(error.to_string());
            message.locked_at = None;
            message.locked_by = None;

            if message.attempts >= 5 {
                message.status = domain::outbox::OutboxMessageStatus::Dead;
            } else {
                message.status = domain::outbox::OutboxMessageStatus::Pending;
                // Exponential backoff
                let delay_seconds = 2_i64.pow(message.attempts as u32) * 60;
                message.next_attempt_at =
                    domain::chrono::Utc::now() + domain::chrono::Duration::seconds(delay_seconds);
            }

            let data_str = serde_json::to_string(&message).unwrap();
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
                SET data = ?,
                    status = ?,
                    next_attempt_at = ?,
                    updated_at = datetime('now')
                WHERE id = ?
                "#,
            )
            .bind(data_str)
            .bind(status_str)
            .bind(message.next_attempt_at)
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
