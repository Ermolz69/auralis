use domain::job::JobId;
use ports::job_runtime_control::{JobRuntimeControlPort, RuntimeCompletion, RuntimeTaskOutcome};
use std::sync::Arc;

pub(crate) struct CompletionGuard {
    runtime: Arc<dyn JobRuntimeControlPort>,
    job_id: JobId,
    completion: Arc<RuntimeCompletion>,
    retain_runtime: bool,
}

impl CompletionGuard {
    pub(crate) fn new(
        runtime: Arc<dyn JobRuntimeControlPort>,
        job_id: JobId,
        completion: Arc<RuntimeCompletion>,
    ) -> Self {
        Self {
            runtime,
            job_id,
            completion,
            retain_runtime: false,
        }
    }

    pub(crate) fn complete(mut self, outcome: RuntimeTaskOutcome) -> RuntimeTaskOutcome {
        self.retain_runtime = outcome == RuntimeTaskOutcome::CleanupUnconfirmed;
        self.completion.record_outcome(outcome)
    }
}

impl Drop for CompletionGuard {
    fn drop(&mut self) {
        if !self.retain_runtime {
            self.runtime.finish_now(&self.job_id);
        }
        self.completion.finish(RuntimeTaskOutcome::RecoveryRequired);
    }
}
