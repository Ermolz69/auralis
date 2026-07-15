use ports::error::PortError;
use ports::repository::FetchPendingResult;

use super::SqliteOutboxRepository;
use crate::sqlite::outbox_mapper::row_to_outbox_message;
use crate::sqlite::outbox_row::OutboxRow;

impl SqliteOutboxRepository {
    pub async fn execute_fetch_pending(
        &self,
        limit: usize,
    ) -> Result<FetchPendingResult, PortError> {
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

        let mut valid_messages = Vec::with_capacity(limit);
        let mut corrupted_isolated = 0;
        let mut isolation_errors = 0;

        // Fetch loop to handle corrupted messages
        loop {
            let remaining = limit - valid_messages.len();
            if remaining == 0 {
                break;
            }

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
            .bind(remaining as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| PortError::Io {
                message: format!("Failed to fetch pending outbox messages: {}", e),
            })?;

            if rows.is_empty() {
                // No more rows to process
                break;
            }

            let fetched_count = rows.len();

            for row in rows {
                let id_raw = row.id.clone();
                match row_to_outbox_message(row) {
                    Ok(message) => valid_messages.push(message),
                    Err(e) => {
                        // Diagnostic reason: e.g. mapping error, invalid payload JSON
                        let reason = format!("Corrupted outbox payload: {}", e);
                        // Mark as dead directly
                        if let Err(err) = self.execute_mark_dead_raw(&id_raw, &reason).await {
                            tracing::error!(
                                "Failed to isolate corrupted outbox message {}: {}",
                                id_raw,
                                err
                            );
                            isolation_errors += 1;
                            // Guarantee exit from fetch loop to avoid infinite loop on this row
                            return Ok(FetchPendingResult {
                                messages: valid_messages,
                                corrupted_isolated,
                                isolation_errors,
                            });
                        } else {
                            corrupted_isolated += 1;
                        }
                    }
                }
            }

            // If we fetched fewer rows than we asked for, there are no more rows in DB
            if fetched_count < remaining {
                break;
            }
        }

        Ok(FetchPendingResult {
            messages: valid_messages,
            corrupted_isolated,
            isolation_errors,
        })
    }
}
