pub mod orphan_writes;
pub mod pair_writes;
pub mod project_writes;
pub mod snapshot;
#[cfg(test)]
mod tests;

use async_trait::async_trait;
use sqlx::SqlitePool;

use ports::error::PortError;
use ports::recovery::{
    FailInterruptedPairCommand, FailOrphanJobCommand, FailProjectWithMissingLinkedJobCommand,
    FailProjectWithoutActiveJobCommand, ReconcileTerminalPairCommand, RecoveryApplyResult,
    RecoverySnapshot, RecoveryStorage,
};

pub struct SqliteRecoveryStorage {
    pool: SqlitePool,
}

impl SqliteRecoveryStorage {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl RecoveryStorage for SqliteRecoveryStorage {
    async fn load_snapshot(&self) -> Result<RecoverySnapshot, PortError> {
        snapshot::load_snapshot(&self.pool).await
    }

    async fn commit_failed_interrupted_pair(
        &self,
        cmd: FailInterruptedPairCommand,
    ) -> Result<RecoveryApplyResult, PortError> {
        pair_writes::commit_failed_interrupted_pair(&self.pool, cmd).await
    }

    async fn commit_reconciled_terminal_pair(
        &self,
        cmd: ReconcileTerminalPairCommand,
    ) -> Result<RecoveryApplyResult, PortError> {
        pair_writes::commit_reconciled_terminal_pair(&self.pool, cmd).await
    }

    async fn commit_failed_project_with_missing_linked_job(
        &self,
        cmd: FailProjectWithMissingLinkedJobCommand,
    ) -> Result<RecoveryApplyResult, PortError> {
        project_writes::commit_failed_project_with_missing_linked_job(&self.pool, cmd).await
    }

    async fn commit_failed_project_without_active_job(
        &self,
        cmd: FailProjectWithoutActiveJobCommand,
    ) -> Result<RecoveryApplyResult, PortError> {
        project_writes::commit_failed_project_without_active_job(&self.pool, cmd).await
    }

    async fn commit_failed_orphan_job(
        &self,
        cmd: FailOrphanJobCommand,
    ) -> Result<RecoveryApplyResult, PortError> {
        orphan_writes::commit_failed_orphan_job(&self.pool, cmd).await
    }
}
