use domain::job::JobId;
use ports::job_runtime_control::{JobRuntimeControlPort, RuntimeCompletion, RuntimeTaskOutcome};
use std::sync::Arc;

pub(super) struct CompletionGuard {
    pub(super) runtime: Arc<dyn JobRuntimeControlPort>,
    pub(super) job_id: JobId,
    pub(super) completion: Arc<RuntimeCompletion>,
}

impl Drop for CompletionGuard {
    fn drop(&mut self) {
        self.runtime.finish_now(&self.job_id);
        self.completion.finish(RuntimeTaskOutcome::RecoveryRequired);
    }
}
