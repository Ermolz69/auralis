use std::time::Duration;

use domain::outbox::OutboxMessageId;
use ports::error::PortError;
use ports::repository::{ClaimReadyResult, OutboxClaim};

use super::SqliteOutboxRepository;
use crate::sqlite::outbox_mapper::row_to_outbox_message;
use crate::sqlite::outbox_row::OutboxRow;

fn storage_error(operation: &'static str, error: sqlx::Error) -> PortError {
    crate::sqlite::helpers::map_sqlite_error(operation, error)
}

impl SqliteOutboxRepository {
    pub(super) async fn execute_claim_ready(
        &self,
        limit: usize,
        lease_timeout: Duration,
    ) -> Result<ClaimReadyResult, PortError> {
        if limit == 0
            || limit > 1024
            || lease_timeout < Duration::from_secs(2)
            || lease_timeout > Duration::from_secs(3600)
        {
            return Err(PortError::InvalidRuntimeConfiguration {
                message: "Outbox claim batch or lease is outside validated bounds".into(),
            });
        }
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|error| storage_error("begin_outbox_claim", error))?;
        sqlx::query(
            "UPDATE outbox_messages SET attempts = attempts + 1,
             status = CASE WHEN kind != 'delete_project_translation' AND attempts + 1 >= 5
               THEN 'dead' ELSE 'failed' END,
             last_error = 'Processing lease expired', locked_by = NULL, locked_at = NULL,
             next_attempt_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
             updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
             WHERE status = 'processing' AND locked_at <=
               strftime('%Y-%m-%dT%H:%M:%SZ', 'now', printf('-%d seconds', ?))",
        )
        .bind(lease_timeout.as_secs() as i64)
        .execute(&mut *tx)
        .await
        .map_err(|error| storage_error("reclaim_outbox_claim", error))?;

        let rows = sqlx::query_as::<_, OutboxRow>(
            "SELECT m.id, m.kind, m.payload_json, m.status, m.attempts,
                    m.next_attempt_at, m.locked_at, m.locked_by, m.last_error,
                    m.deduplication_key, m.created_at, m.updated_at,
                    m.aggregate_type, m.aggregate_id
             FROM outbox_messages m
             WHERE m.status IN ('pending', 'failed')
               AND m.next_attempt_at <= strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
               AND NOT EXISTS (
                 SELECT 1 FROM outbox_messages predecessor
                 WHERE COALESCE(predecessor.sequence, predecessor.rowid) < COALESCE(m.sequence, m.rowid)
                   AND predecessor.status != 'done'
                   AND (COALESCE(predecessor.conflict_group, '*') = '*'
                     OR COALESCE(m.conflict_group, '*') = '*'
                     OR predecessor.conflict_group = m.conflict_group)
               )
             ORDER BY COALESCE(m.sequence, m.rowid) LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&mut *tx)
        .await
        .map_err(|error| storage_error("select_ready_outbox", error))?;

        let mut claims = Vec::with_capacity(rows.len());
        let mut corrupted_isolated = 0;
        for row in rows {
            let raw_id = row.id.clone();
            let message = match row_to_outbox_message(row) {
                Ok(message) => message,
                Err(error) => {
                    sqlx::query("UPDATE outbox_messages SET status = 'dead', last_error = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ? AND status IN ('pending', 'failed')")
                        .bind(format!("Corrupted outbox payload: {error}"))
                        .bind(raw_id)
                        .execute(&mut *tx).await
                        .map_err(|error| storage_error("isolate_outbox_row", error))?;
                    corrupted_isolated += 1;
                    continue;
                }
            };
            let token = uuid::Uuid::new_v4().to_string();
            let claimed = sqlx::query(
                "UPDATE outbox_messages SET status = 'processing', locked_by = ?,
                 locked_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                 updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                 WHERE id = ? AND status IN ('pending', 'failed')",
            )
            .bind(&token)
            .bind(message.id.to_string())
            .execute(&mut *tx)
            .await
            .map_err(|error| storage_error("claim_outbox_row", error))?;
            if claimed.rows_affected() == 1 {
                claims.push(OutboxClaim { message, token });
            }
        }
        tx.commit()
            .await
            .map_err(|error| storage_error("commit_outbox_claim", error))?;
        Ok(ClaimReadyResult {
            claims,
            corrupted_isolated,
            isolation_errors: 0,
        })
    }

    pub(super) async fn execute_renew_claim(
        &self,
        id: &OutboxMessageId,
        token: &str,
    ) -> Result<bool, PortError> {
        let result = sqlx::query("UPDATE outbox_messages SET locked_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ? AND status = 'processing' AND locked_by = ?")
            .bind(id.to_string()).bind(token).execute(&self.pool).await
            .map_err(|error| storage_error("renew_outbox_claim", error))?;
        Ok(result.rows_affected() == 1)
    }

    pub(super) async fn execute_claim_is_current(
        &self,
        id: &OutboxMessageId,
        token: &str,
    ) -> Result<bool, PortError> {
        let current: i64 = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM outbox_messages WHERE id = ? AND status = 'processing' AND locked_by = ?)")
            .bind(id.to_string()).bind(token).fetch_one(&self.pool).await
            .map_err(|error| storage_error("check_outbox_claim", error))?;
        Ok(current == 1)
    }

    pub(super) async fn execute_mark_done_claim(
        &self,
        id: &OutboxMessageId,
        token: &str,
    ) -> Result<bool, PortError> {
        let result = sqlx::query("UPDATE outbox_messages SET status = 'done', locked_at = NULL, locked_by = NULL, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now') WHERE id = ? AND status = 'processing' AND locked_by = ?")
            .bind(id.to_string()).bind(token).execute(&self.pool).await
            .map_err(|error| storage_error("ack_outbox_claim", error))?;
        Ok(result.rows_affected() == 1)
    }

    pub(super) async fn execute_mark_failed_claim(
        &self,
        id: &OutboxMessageId,
        token: &str,
        error: &str,
    ) -> Result<bool, PortError> {
        let result = sqlx::query(
            "UPDATE outbox_messages SET attempts = attempts + 1, last_error = ?,
             status = CASE WHEN kind != 'delete_project_translation' AND attempts + 1 >= 5
               THEN 'dead' ELSE 'failed' END,
             next_attempt_at = CASE WHEN kind != 'delete_project_translation' AND attempts + 1 >= 5
               THEN next_attempt_at ELSE strftime('%Y-%m-%dT%H:%M:%SZ', 'now',
                 '+' || CAST(1 << MIN(attempts + 1, 6) AS TEXT) || ' minutes') END,
             locked_at = NULL, locked_by = NULL,
             updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
             WHERE id = ? AND status = 'processing' AND locked_by = ?",
        )
        .bind(error)
        .bind(id.to_string())
        .bind(token)
        .execute(&self.pool)
        .await
        .map_err(|error| storage_error("fail_outbox_claim", error))?;
        Ok(result.rows_affected() == 1)
    }
}
