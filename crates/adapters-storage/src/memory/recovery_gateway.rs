use async_trait::async_trait;
use ports::error::PortError;
use ports::recovery::{
    FailInterruptedPairCommand, FailOrphanJobCommand, FailProjectWithMissingLinkedJobCommand,
    FailProjectWithoutActiveJobCommand, ReconcileTerminalPairCommand, RecoveryApplyResult,
    RecoverySnapshot, RecoveryStorage,
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

    async fn commit_failed_project_with_missing_linked_job(
        &self,
        _cmd: FailProjectWithMissingLinkedJobCommand,
    ) -> Result<RecoveryApplyResult, PortError> {
        Ok(RecoveryApplyResult::Applied)
    }

    async fn commit_failed_project_without_active_job(
        &self,
        _cmd: FailProjectWithoutActiveJobCommand,
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
