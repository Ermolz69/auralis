use super::{ManagedTranslationAttempt, TranslationRunRequest, TranslationRunUseCase};
use crate::error::ApplicationError;
use crate::usecases::translation::pending_job_failure;
use domain::job::JobId;
use ports::cancellation::CancelHandle;
use ports::error::PortError;
use ports::job_runtime_control::{
    JobRuntimeControlPort, RuntimeCompletion, RuntimeTask, RuntimeTaskOutcome,
};
use ports::translation_host_job::TranslationHostJobStore;
use std::sync::Arc;
use tokio::sync::oneshot;
use uuid::Uuid;

struct CompletionGuard {
    job_id: JobId,
    runtime: Arc<dyn JobRuntimeControlPort>,
    completion: Arc<RuntimeCompletion>,
}

impl Drop for CompletionGuard {
    fn drop(&mut self) {
        self.runtime.finish_now(&self.job_id);
        self.completion.finish(RuntimeTaskOutcome::RecoveryRequired);
    }
}

pub(super) fn spawn(
    admitted: ManagedTranslationAttempt,
    request: TranslationRunRequest,
    runtime: Arc<dyn JobRuntimeControlPort>,
    jobs: Arc<dyn TranslationHostJobStore>,
    pause: TranslationRunUseCase,
) -> (RuntimeTask, oneshot::Sender<()>) {
    let job_id = admitted.job().id().clone();
    let run_id = admitted.run_id();
    let (cancel_handle, token) = CancelHandle::new();
    let (release, released) = oneshot::channel();
    let completion = Arc::new(RuntimeCompletion::new());
    let completion_for_task = completion.clone();
    let join_handle = tokio::spawn(async move {
        let guard = CompletionGuard {
            job_id: job_id.clone(),
            runtime,
            completion: completion_for_task,
        };
        let released = tokio::select! {
            release = released => release.is_ok(),
            _ = token.cancelled() => false,
        };
        if !released {
            log_pending_failure(jobs.as_ref(), run_id, &job_id).await;
            return guard
                .completion
                .record_outcome(RuntimeTaskOutcome::Cancelled);
        }
        let execution = admitted.execute();
        tokio::pin!(execution);
        let result = tokio::select! {
            result = &mut execution => result,
            _ = token.cancelled() => {
                if let Err(error) = pause.pause(request).await {
                    tracing::warn!(job_id = %job_id, error = %error, "translation pause request failed during runtime cancellation");
                }
                execution.await
            }
        };
        let outcome = match result {
            Ok(_) => RuntimeTaskOutcome::Completed,
            Err(error) => {
                log_pending_failure(jobs.as_ref(), run_id, &job_id).await;
                if token.is_cancelled()
                    || matches!(&error, ApplicationError::Port(PortError::Cancelled))
                {
                    RuntimeTaskOutcome::Cancelled
                } else {
                    tracing::warn!(job_id = %job_id, error = %error, "managed translation attempt did not publish");
                    RuntimeTaskOutcome::ApplicationFailed
                }
            }
        };
        guard.completion.record_outcome(outcome)
    });
    (
        RuntimeTask {
            cancel: cancel_handle,
            join_handle,
            completion,
        },
        release,
    )
}

async fn log_pending_failure(jobs: &dyn TranslationHostJobStore, run_id: Uuid, job_id: &JobId) {
    if let Err(error) = pending_job_failure::close_pending(jobs, run_id, job_id).await {
        tracing::error!(job_id = %job_id, error = %error, "pending translation job cleanup needs recovery");
    }
}
