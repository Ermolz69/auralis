use sqlx::{Pool, Sqlite};

mod artifact_recovery;
pub mod claim;
pub mod mutations;
mod parallel_claim;
mod protected_resources;
pub mod queries;

#[cfg(test)]
mod parallel_tests;
#[cfg(test)]
mod payload_compatibility_tests;
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
use ports::repository::{
    ClaimReadyResult, FetchPendingResult, OutboxProtectedResources, OutboxRepository,
};

#[async_trait]
impl OutboxRepository for SqliteOutboxRepository {
    async fn protected_resources(&self) -> Result<OutboxProtectedResources, PortError> {
        self.execute_protected_resources().await
    }
    async fn claim_ready(
        &self,
        limit: usize,
        lease_timeout: std::time::Duration,
    ) -> Result<ClaimReadyResult, PortError> {
        self.execute_claim_ready(limit, lease_timeout).await
    }
    async fn renew_claim(&self, id: &OutboxMessageId, token: &str) -> Result<bool, PortError> {
        self.execute_renew_claim(id, token).await
    }
    async fn claim_is_current(&self, id: &OutboxMessageId, token: &str) -> Result<bool, PortError> {
        self.execute_claim_is_current(id, token).await
    }
    async fn mark_done_claim(&self, id: &OutboxMessageId, token: &str) -> Result<bool, PortError> {
        self.execute_mark_done_claim(id, token).await
    }
    async fn mark_failed_claim(
        &self,
        id: &OutboxMessageId,
        token: &str,
        error: &str,
    ) -> Result<bool, PortError> {
        self.execute_mark_failed_claim(id, token, error).await
    }
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
