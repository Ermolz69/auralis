#![allow(clippy::unwrap_used, clippy::expect_used)]
use sqlx::{Pool, Sqlite};

pub mod claim;
pub mod mutations;
pub mod queries;

#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct SqliteOutboxRepository {
    pub(crate) pool: Pool<Sqlite>,
}

impl SqliteOutboxRepository {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }
}

use async_trait::async_trait;
use domain::outbox::OutboxMessageId;
use ports::error::PortError;
use ports::repository::{FetchPendingResult, OutboxRepository};

#[async_trait]
impl OutboxRepository for SqliteOutboxRepository {
    async fn fetch_pending(&self, limit: usize) -> Result<FetchPendingResult, PortError> {
        self.execute_fetch_pending(limit).await
    }

    async fn mark_processing(
        &self,
        id: &OutboxMessageId,
        locked_by: &str,
    ) -> Result<bool, PortError> {
        self.execute_mark_processing(id, locked_by).await
    }

    async fn mark_done(&self, id: &OutboxMessageId) -> Result<(), PortError> {
        self.execute_mark_done(id).await
    }

    async fn mark_failed(&self, id: &OutboxMessageId, error: &str) -> Result<(), PortError> {
        self.execute_mark_failed(id, error).await
    }

    async fn mark_dead_raw(&self, id_raw: &str, reason: &str) -> Result<(), PortError> {
        self.execute_mark_dead_raw(id_raw, reason).await
    }

    async fn prune_terminal_rows(
        &self,
        done_before: chrono::DateTime<chrono::Utc>,
        dead_before: chrono::DateTime<chrono::Utc>,
        batch_limit: u32,
    ) -> Result<ports::repository::OutboxPruneReport, PortError> {
        self.execute_prune_terminal_rows(done_before, dead_before, batch_limit)
            .await
    }
}
