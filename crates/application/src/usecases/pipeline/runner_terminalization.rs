use domain::job::JobId;
use ports::cancellation::CancellationToken;
use ports::error::PortError;
use ports::job_runtime_control::RuntimeTaskOutcome;
use ports::job_scheduler::JobSchedulerPort;
use std::future::Future;

#[derive(Debug, Clone, Copy)]
pub(crate) struct TerminalFailure {
    code: &'static str,
    message: &'static str,
    status: &'static str,
}

impl TerminalFailure {
    pub(crate) fn panic() -> Self {
        Self {
            code: "RUNNER_PANIC",
            message: "Pipeline runner terminated unexpectedly.",
            status: "panicked",
        }
    }

    pub(crate) fn stage_update() -> Self {
        Self {
            code: "STAGE_UPDATE_FAILED",
            message: "Pipeline progress could not be persisted.",
            status: "stage_update_failed",
        }
    }

    pub(crate) fn subtitle_import() -> Self {
        Self {
            code: "SUBTITLE_IMPORT_FAILED",
            message: "Subtitle import failed.",
            status: "subtitle_import_failed",
        }
    }
}

pub(crate) async fn terminalize_runner_failure(
    job_scheduler: &dyn JobSchedulerPort,
    job_id: &JobId,
    failure: TerminalFailure,
    guard: &mut crate::observability::execution_summary::ExecutionSummaryGuard,
) -> RuntimeTaskOutcome {
    match job_scheduler
        .fail_job(job_id, failure.code.into(), failure.message.into(), false)
        .await
    {
        Ok(_) => {
            guard.summary.update_status(failure.status);
            RuntimeTaskOutcome::ApplicationFailed
        }
        Err(PortError::NotFound { .. }) => {
            guard.summary.update_status("deleted");
            RuntimeTaskOutcome::DeletedNoOp
        }
        Err(_) => {
            guard.summary.update_status("recovery_required");
            RuntimeTaskOutcome::RecoveryRequired
        }
    }
}

pub(crate) fn cancelled(
    guard: &mut crate::observability::execution_summary::ExecutionSummaryGuard,
) -> RuntimeTaskOutcome {
    guard.summary.update_status("cancelled");
    RuntimeTaskOutcome::Cancelled
}

pub(crate) async fn await_or_cancel<T, F>(
    token: &CancellationToken,
    future: F,
) -> Result<T, RuntimeTaskOutcome>
where
    F: Future<Output = T>,
{
    tokio::pin!(future);
    tokio::select! {
        biased;
        _ = token.cancelled() => Err(RuntimeTaskOutcome::Cancelled),
        result = &mut future => Ok(result),
    }
}
