use domain::job::JobId;
use domain::project::ProjectId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinHandle;

use crate::error::PortError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeTaskOutcome {
    Completed,
    Cancelled,
    DeletedNoOp,
    ApplicationFailed,
    RecoveryRequired,
    Panicked,
}

pub struct RuntimeCompletion {
    outcome: std::sync::Mutex<Option<RuntimeTaskOutcome>>,
    completed: tokio_util::sync::CancellationToken,
}

impl Default for RuntimeCompletion {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeCompletion {
    pub fn new() -> Self {
        Self {
            outcome: std::sync::Mutex::new(None),
            completed: tokio_util::sync::CancellationToken::new(),
        }
    }

    pub fn record_outcome(&self, outcome: RuntimeTaskOutcome) -> RuntimeTaskOutcome {
        let mut current = self
            .outcome
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *current = Some(outcome);
        outcome
    }

    pub fn finish(&self, fallback: RuntimeTaskOutcome) {
        let mut current = self
            .outcome
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if current.is_none() {
            *current = Some(fallback);
        }
        drop(current);
        self.completed.cancel();
    }

    pub async fn wait(&self) -> RuntimeTaskOutcome {
        self.completed.cancelled().await;
        self.outcome
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .unwrap_or(RuntimeTaskOutcome::RecoveryRequired)
    }
}

pub struct RuntimeTask {
    pub cancel: crate::cancellation::CancelHandle,
    pub join_handle: JoinHandle<RuntimeTaskOutcome>,
    pub completion: Arc<RuntimeCompletion>,
}

impl std::fmt::Debug for RuntimeTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuntimeTask").finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct AttachTaskError {
    pub source: PortError,
    pub task: RuntimeTask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeCleanupOutcome {
    Missing,
    ReservationRemoved,
    Completed,
    CooperativeCancelled,
    ForcedAborted,
    Panicked,
    JoinFailed,
    Unconfirmed,
}

#[derive(Debug, Clone)]
pub struct RuntimeCleanupReport {
    pub jobs: HashMap<JobId, RuntimeCleanupOutcome>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeShutdownReport {
    pub completed_count: usize,
    pub reservation_removed_count: usize,
    pub cooperative_cancelled_count: usize,
    pub forced_aborted_count: usize,
    pub panicked_count: usize,
    pub join_failed_count: usize,
    pub unconfirmed_count: usize,
    pub cleanup_deferred_count: usize,
}

#[async_trait::async_trait]
pub trait JobRuntimeControlPort: Send + Sync {
    async fn reserve(&self, job_id: JobId, project_id: ProjectId) -> Result<(), PortError>;

    async fn attach_task(&self, job_id: JobId, task: RuntimeTask) -> Result<(), AttachTaskError>;

    fn finish_now(&self, job_id: &JobId);

    async fn rollback_runtime_start(
        &self,
        job_id: &JobId,
    ) -> Result<RuntimeCleanupOutcome, PortError>;

    async fn cancel_and_evict_jobs(
        &self,
        job_ids: &[JobId],
    ) -> Result<RuntimeCleanupReport, PortError>;

    async fn drain_all(
        &self,
        _deadline: std::time::Duration,
    ) -> Result<RuntimeShutdownReport, PortError> {
        Ok(RuntimeShutdownReport::default())
    }
}

#[cfg(test)]
mod tests {
    use super::{RuntimeCompletion, RuntimeTaskOutcome};
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn cancellation_completion_is_sticky_for_late_and_concurrent_waiters() {
        let completion = Arc::new(RuntimeCompletion::new());
        completion.record_outcome(RuntimeTaskOutcome::Cancelled);
        completion.finish(RuntimeTaskOutcome::RecoveryRequired);

        let late = tokio::time::timeout(Duration::from_millis(100), completion.wait()).await;
        assert_eq!(late, Ok(RuntimeTaskOutcome::Cancelled));

        let waiters = (0..256).map(|_| {
            let completion = completion.clone();
            tokio::spawn(async move { completion.wait().await })
        });
        for waiter in waiters {
            assert!(matches!(waiter.await, Ok(RuntimeTaskOutcome::Cancelled)));
        }
    }

    #[tokio::test]
    async fn cancellation_completion_uses_fallback_when_task_has_no_outcome() {
        let completion = RuntimeCompletion::new();
        completion.finish(RuntimeTaskOutcome::RecoveryRequired);

        assert_eq!(
            completion.wait().await,
            RuntimeTaskOutcome::RecoveryRequired
        );
    }
}
