use domain::outbox::OutboxMessageId;
use ports::error::PortError;

use super::SqliteOutboxRepository;

impl SqliteOutboxRepository {
    pub async fn execute_mark_done(&self, id: &OutboxMessageId) -> Result<(), PortError> {
        let id_str = id.to_string();

        let status_opt: Option<String> =
            sqlx::query_scalar("SELECT status FROM outbox_messages WHERE id = ?")
                .bind(&id_str)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| PortError::Io {
                    message: format!("Failed to query status of outbox message {}: {}", id, e),
                })?;

        let status = match status_opt {
            Some(s) => s,
            None => {
                return Err(PortError::Io {
                    message: format!("Outbox message {} not found or already modified", id),
                });
            }
        };

        if status == "done" || status == "dead" {
            return Ok(());
        }

        if status != "processing" {
            return Err(PortError::Io {
                message: format!("Outbox message {} not found or already modified", id),
            });
        }

        let result = sqlx::query(
            r#"
            UPDATE outbox_messages
            SET status = 'done',
                locked_at = NULL,
                locked_by = NULL,
                updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
            WHERE id = ? AND status = 'processing' AND sequence IS NULL
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
                    WHEN kind = 'delete_project_translation' THEN 'failed'
                    WHEN attempts + 1 >= 5 THEN 'dead'
                    ELSE 'failed'
                END,
                next_attempt_at = CASE 
                    WHEN kind = 'delete_project_translation' THEN strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '+' || CAST(CAST(1 AS INTEGER) << MIN(attempts + 1, 6) AS TEXT) || ' minutes')
                    WHEN attempts + 1 >= 5 THEN next_attempt_at
                    ELSE strftime('%Y-%m-%dT%H:%M:%SZ', 'now', '+' || CAST(CAST(1 AS INTEGER) << (attempts + 1) AS TEXT) || ' minutes')
                END
            WHERE id = ? AND status = 'processing' AND sequence IS NULL
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
            WHERE id = ? AND status IN ('pending', 'failed')
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

    pub async fn execute_prune_terminal_rows(
        &self,
        done_before: chrono::DateTime<chrono::Utc>,
        dead_before: chrono::DateTime<chrono::Utc>,
        batch_limit: u32,
    ) -> Result<ports::repository::OutboxPruneReport, PortError> {
        let mut tx = self.pool.begin().await.map_err(|e| PortError::Io {
            message: format!("Failed to begin prune transaction: {}", e),
        })?;

        let done_cutoff = done_before.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let dead_cutoff = dead_before.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

        let done_result = sqlx::query(
            r#"
            DELETE FROM outbox_messages
            WHERE id IN (
                SELECT completed.id FROM outbox_messages completed
                WHERE completed.status = 'done'
                  AND completed.updated_at < ?
                  AND NOT EXISTS (
                    SELECT 1 FROM outbox_messages dependent
                    WHERE COALESCE(dependent.sequence, dependent.rowid) > COALESCE(completed.sequence, completed.rowid)
                      AND dependent.status != 'done'
                      AND (COALESCE(dependent.conflict_group, '*') = '*'
                        OR COALESCE(completed.conflict_group, '*') = '*'
                        OR dependent.conflict_group = completed.conflict_group)
                  )
                ORDER BY completed.updated_at ASC, completed.id ASC
                LIMIT ?
            )
            "#,
        )
        .bind(&done_cutoff)
        .bind(batch_limit)
        .execute(&mut *tx)
        .await
        .map_err(|e| PortError::Io {
            message: format!("Failed to prune done outbox messages: {}", e),
        })?;

        let dead_result = sqlx::query(
            r#"
            DELETE FROM outbox_messages
            WHERE id IN (
                SELECT dead.id FROM outbox_messages dead
                WHERE dead.status = 'dead'
                  AND dead.updated_at < ?
                  AND NOT EXISTS (
                    SELECT 1 FROM outbox_messages dependent
                    WHERE COALESCE(dependent.sequence, dependent.rowid) > COALESCE(dead.sequence, dead.rowid)
                      AND dependent.status != 'done'
                      AND (COALESCE(dependent.conflict_group, '*') = '*'
                        OR COALESCE(dead.conflict_group, '*') = '*'
                        OR dependent.conflict_group = dead.conflict_group)
                  )
                  AND CASE WHEN dead.kind = 'finalize_staged_artifact' THEN
                    CASE WHEN json_valid(dead.payload_json) THEN
                      json_extract(dead.payload_json, '$.artifact_id') IS NOT NULL
                      AND NOT EXISTS (
                        SELECT 1 FROM artifacts a
                        JOIN projects p ON p.id = a.project_id
                        WHERE a.id = json_extract(dead.payload_json, '$.artifact_id')
                          AND a.state = 'pending_finalize'
                      )
                    ELSE 0 END
                  ELSE 1 END
                ORDER BY dead.updated_at ASC, dead.id ASC
                LIMIT ?
            )
            "#,
        )
        .bind(&dead_cutoff)
        .bind(batch_limit)
        .execute(&mut *tx)
        .await
        .map_err(|e| PortError::Io {
            message: format!("Failed to prune dead outbox messages: {}", e),
        })?;

        tx.commit().await.map_err(|e| PortError::Io {
            message: format!("Failed to commit prune transaction: {}", e),
        })?;

        Ok(ports::repository::OutboxPruneReport {
            done_deleted: done_result.rows_affected() as usize,
            dead_deleted: dead_result.rows_affected() as usize,
        })
    }
}
