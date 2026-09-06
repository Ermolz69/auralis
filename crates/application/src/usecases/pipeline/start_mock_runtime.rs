use domain::job::JobId;
use domain::project::ProjectId;
use ports::cancellation::{CancelHandle, CancellationToken};
use ports::job_runtime_control::{
    JobRuntimeControlPort, RuntimeCompletion, RuntimeTask, RuntimeTaskOutcome,
};
use ports::job_scheduler::JobSchedulerPort;
use ports::repository::ProjectRepository;
use ports::source::SubtitleSourcePort;
use ports::storage::ArtifactStore;
use ports::transaction::StorageUnitOfWork;
use std::sync::Arc;
use tracing::Instrument;

use super::mock_dubbing_pipeline::MockDubbingPipelineRunner;
use super::runner_terminalization::{TerminalFailure, terminalize_runner_failure};

pub(super) struct CompletionGuard {
    job_id: JobId,
    completion: Arc<RuntimeCompletion>,
    job_runtime: Arc<dyn JobRuntimeControlPort>,
}

impl CompletionGuard {
    pub(super) fn new(
        job_id: JobId,
        completion: Arc<RuntimeCompletion>,
        job_runtime: Arc<dyn JobRuntimeControlPort>,
    ) -> Self {
        Self {
            job_id,
            completion,
            job_runtime,
        }
    }

    pub(super) fn record_outcome(&self, outcome: RuntimeTaskOutcome) -> RuntimeTaskOutcome {
        let mut outcome_guard = self
            .completion
            .outcome
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        *outcome_guard = Some(outcome);
        outcome
    }
}

impl Drop for CompletionGuard {
    fn drop(&mut self) {
        let mut outcome_guard = self
            .completion
            .outcome
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        if outcome_guard.is_none() {
            *outcome_guard = Some(RuntimeTaskOutcome::RecoveryRequired);
        }
        self.completion
            .state
            .store(1, std::sync::atomic::Ordering::Release);
        self.completion.notify.notify_waiters();
        self.job_runtime.finish_now(&self.job_id);
    }
}

pub(super) async fn await_release_gate(
    release_rx: tokio::sync::oneshot::Receiver<()>,
    token: &CancellationToken,
    guard: &mut crate::observability::execution_summary::ExecutionSummaryGuard,
) -> Result<(), RuntimeTaskOutcome> {
    let mut release_fut = std::pin::pin!(release_rx);
    loop {
        if token.is_cancelled() {
            guard.summary.update_status("cancelled_before_release");
            return Err(RuntimeTaskOutcome::Cancelled);
        }
        tokio::select! {
            res = &mut release_fut => {
                if res.is_err() {
                    guard.summary.update_status("cancelled_at_release");
                    return Err(RuntimeTaskOutcome::Cancelled);
                }
                return Ok(());
            }
            _ = token.cancelled() => {
                guard.summary.update_status("cancelled_before_release");
                return Err(RuntimeTaskOutcome::Cancelled);
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {}
        }
    }
}

pub(super) struct RuntimeStartControl {
    pub(super) activate: tokio::sync::oneshot::Sender<()>,
    pub(super) acknowledged: tokio::sync::oneshot::Receiver<()>,
    pub(super) release: tokio::sync::oneshot::Sender<()>,
}

pub(super) struct RuntimeStartGates {
    activate: tokio::sync::oneshot::Receiver<()>,
    acknowledge: tokio::sync::oneshot::Sender<()>,
    release: tokio::sync::oneshot::Receiver<()>,
}

pub(super) fn runtime_start_handshake() -> (RuntimeStartControl, RuntimeStartGates) {
    let (activate, activate_rx) = tokio::sync::oneshot::channel();
    let (acknowledge, acknowledged) = tokio::sync::oneshot::channel();
    let (release, release_rx) = tokio::sync::oneshot::channel();
    (
        RuntimeStartControl {
            activate,
            acknowledged,
            release,
        },
        RuntimeStartGates {
            activate: activate_rx,
            acknowledge,
            release: release_rx,
        },
    )
}

pub(super) struct MockPipelineTaskSpec<
    R: ProjectRepository + Clone + 'static,
    V: SubtitleSourcePort + Clone + 'static,
    T: StorageUnitOfWork + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> {
    pub(super) runner: MockDubbingPipelineRunner<R, V, T, S>,
    pub(super) job_scheduler: Arc<dyn JobSchedulerPort>,
    pub(super) job_runtime: Arc<dyn JobRuntimeControlPort>,
    pub(super) job_id: JobId,
    pub(super) project_id: ProjectId,
    pub(super) cancel_handle: CancelHandle,
    pub(super) token: CancellationToken,
    pub(super) completion: Arc<RuntimeCompletion>,
    pub(super) gates: RuntimeStartGates,
    pub(super) span: tracing::Span,
}

pub(super) fn spawn_mock_pipeline_task<
    R: ProjectRepository + Clone + 'static,
    V: SubtitleSourcePort + Clone + 'static,
    T: StorageUnitOfWork + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
>(
    spec: MockPipelineTaskSpec<R, V, T, S>,
) -> RuntimeTask {
    let MockPipelineTaskSpec {
        runner,
        job_scheduler,
        job_runtime,
        job_id,
        project_id,
        cancel_handle,
        token,
        completion,
        gates,
        span,
    } = spec;
    let span_clone_for_spawn = span.clone();
    let completion_for_task = completion.clone();
    let wrapper = async move {
        let mut guard = crate::observability::execution_summary::ExecutionSummaryGuard::new(
            span.clone(),
            crate::observability::execution_summary::OperationSummary::JobExecution {
                project_id: project_id.to_string(),
                job_id: job_id.to_string(),
                action: "job_execution",
                status: "aborted".to_string(),
            },
        );

        let completion_guard =
            CompletionGuard::new(job_id.clone(), completion_for_task, job_runtime);

        if gates.activate.await.is_err() {
            guard.summary.update_status("cancelled_at_activate");
            return completion_guard.record_outcome(RuntimeTaskOutcome::Cancelled);
        }
        if gates.acknowledge.send(()).is_err() {
            guard.summary.update_status("cancelled_at_ack");
            return completion_guard.record_outcome(RuntimeTaskOutcome::Cancelled);
        }
        if let Err(outcome) = await_release_gate(gates.release, &token, &mut guard).await {
            return completion_guard.record_outcome(outcome);
        }

        let actual_run = async {
            runner
                .run(job_id.clone(), project_id, token, &mut guard)
                .await
        };
        let outcome = match futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(
            actual_run,
        ))
        .await
        {
            Ok(o) => o,
            Err(_) => {
                terminalize_runner_failure(
                    job_scheduler.as_ref(),
                    &job_id,
                    TerminalFailure::panic(),
                    &mut guard,
                )
                .await
            }
        };

        completion_guard.record_outcome(outcome)
    };

    RuntimeTask {
        cancel: cancel_handle,
        join_handle: tokio::spawn(wrapper.instrument(span_clone_for_spawn)),
        completion,
    }
}
