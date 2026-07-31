use async_trait::async_trait;
use ports::error::PortError;
use ports::recovery::{
    FailInterruptedPairCommand, FailLegacyPairFallbackCommand, FailLegacyProjectWithoutJobCommand,
    FailOrphanJobCommand, FailProjectWithMissingLinkedJobCommand, ReconcileTerminalPairCommand,
    RecoveryApplyResult, RecoverySnapshot, RecoveryStorage,
};

#[derive(Clone, Default)]
pub struct InMemoryRecoveryStorage;

impl InMemoryRecoveryStorage {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RecoveryStorage for InMemoryRecoveryStorage {
    async fn load_snapshot(&self) -> Result<RecoverySnapshot, PortError> {
        // Dev-only fallback
        Ok(RecoverySnapshot {
            processing_projects: vec![],
            linked_jobs: vec![],
            active_jobs: vec![],
        })
    }

    async fn commit_failed_interrupted_pair(
        &self,
        _cmd: FailInterruptedPairCommand,
    ) -> Result<RecoveryApplyResult, PortError> {
        Ok(RecoveryApplyResult::Applied)
    }

    async fn commit_reconciled_terminal_pair(
        &self,
        _cmd: ReconcileTerminalPairCommand,
    ) -> Result<RecoveryApplyResult, PortError> {
        Ok(RecoveryApplyResult::Applied)
    }

    async fn commit_legacy_pair_fallback(
        &self,
        _cmd: FailLegacyPairFallbackCommand,
    ) -> Result<RecoveryApplyResult, PortError> {
        Ok(RecoveryApplyResult::Applied)
    }

    async fn commit_failed_project_with_missing_linked_job(
        &self,
        _cmd: FailProjectWithMissingLinkedJobCommand,
    ) -> Result<RecoveryApplyResult, PortError> {
        Ok(RecoveryApplyResult::Applied)
    }

    async fn commit_failed_legacy_project_without_job(
        &self,
        _cmd: FailLegacyProjectWithoutJobCommand,
    ) -> Result<RecoveryApplyResult, PortError> {
        Ok(RecoveryApplyResult::Applied)
    }

    async fn commit_failed_orphan_job(
        &self,
        _cmd: FailOrphanJobCommand,
    ) -> Result<RecoveryApplyResult, PortError> {
        Ok(RecoveryApplyResult::Applied)
    }
}
