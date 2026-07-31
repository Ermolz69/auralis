use domain::outbox::OutboxMessageId;
use ports::error::PortError;

use super::SqliteOutboxRepository;

impl SqliteOutboxRepository {
    pub async fn execute_mark_processing(
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
}
