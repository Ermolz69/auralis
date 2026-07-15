use domain::outbox::OutboxMessageId;
use ports::error::PortError;

use super::SqliteOutboxRepository;

impl SqliteOutboxRepository {
    pub async fn execute_mark_done(&self, id: &OutboxMessageId) -> Result<(), PortError> {
        let id_str = id.to_string();
        let result = sqlx::query(
            r#"
            UPDATE outbox_messages
            SET status = 'done',
                locked_at = NULL,
                locked_by = NULL,
                updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
            WHERE id = ?
            "#,
        )
        .bind(&id_str)
        .execute(&self.pool)
        .await
        .map_err(|e| PortError::Io {
            message: format!("Failed to mark outbox message {} as done: {}", id, e),
        })?;

        if result.rows_affected() == 0 {
            return Err(PortError::Io {
                message: format!("Outbox message {} not found or already modified", id),
            });
        }

        Ok(())
    }

    pub async fn execute_mark_failed(
        &self,
        id: &OutboxMessageId,
        error: &str,
    ) -> Result<(), PortError> {
        let id_str = id.to_string();

        let result = sqlx::query(
            r#"
            UPDATE outbox_messages
            SET 
                attempts = attempts + 1,
                last_error = ?,
                locked_at = NULL,
                locked_by = NULL,
                updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                status = CASE 
                    WHEN attempts + 1 >= 5 THEN 'dead'
                    ELSE 'failed'
                END,
                next_attempt_at = CASE 
                    WHEN attempts + 1 >= 5 THEN next_attempt_at
                    ELSE strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '+' || CAST(CAST(1 AS INTEGER) << (attempts + 1) AS TEXT) || ' minutes')
                END
            WHERE id = ?
            "#,
        )
        .bind(error)
        .bind(&id_str)
        .execute(&self.pool)
        .await
        .map_err(|e| PortError::Io {
            message: format!("Failed to update outbox message {}: {}", id, e),
        })?;

        if result.rows_affected() == 0 {
            return Err(PortError::Io {
                message: format!("Outbox message {} not found or already modified", id),
            });
        }

        Ok(())
    }

    pub async fn execute_mark_dead_raw(&self, id_raw: &str, reason: &str) -> Result<(), PortError> {
        sqlx::query(
            r#"
            UPDATE outbox_messages
            SET status = 'dead',
                last_error = ?,
                locked_at = NULL,
                locked_by = NULL,
                updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
            WHERE id = ?
            "#,
        )
        .bind(reason)
        .bind(id_raw)
        .execute(&self.pool)
        .await
        .map_err(|e| PortError::Io {
            message: format!(
                "Failed to mark raw outbox message {} as dead: {}",
                id_raw, e
            ),
        })?;

        Ok(())
    }
}
