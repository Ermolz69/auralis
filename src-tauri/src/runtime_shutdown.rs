#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RuntimeLifecycleAction {
    Ignore,
    FinalShutdown,
}

pub(crate) fn classify_run_event(event: &tauri::RunEvent) -> RuntimeLifecycleAction {
    match event {
        tauri::RunEvent::Exit => RuntimeLifecycleAction::FinalShutdown,
        _ => RuntimeLifecycleAction::Ignore,
    }
}

pub trait TracingShutdown {
    fn shutdown(self, timeout: std::time::Duration) -> TracingShutdownOutcome;
}

impl TracingShutdown for crate::observability::init::TracingGuard {
    fn shutdown(self, timeout: std::time::Duration) -> TracingShutdownOutcome {
        crate::observability::init::TracingGuard::shutdown(self, timeout)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerOutcome {
    Graceful,
    Aborted,
    JoinFailed,
    AlreadyStopped,
    SignalFailed,
}

impl WorkerOutcome {
    pub fn is_graceful(&self) -> bool {
        matches!(
            self,
            WorkerOutcome::Graceful | WorkerOutcome::AlreadyStopped
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TracingShutdownOutcome {
    Flushed,
    TimedOut,
    NotOwned,
    FlushThreadStartFailed,
}

impl TracingShutdownOutcome {
    pub fn is_graceful(&self) -> bool {
        matches!(
            self,
            TracingShutdownOutcome::Flushed | TracingShutdownOutcome::NotOwned
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerShutdownReport {
    pub outbox_outcome: WorkerOutcome,
    pub bridge_outcome: WorkerOutcome,
}

impl WorkerShutdownReport {
    pub fn is_graceful(&self) -> bool {
        self.outbox_outcome.is_graceful() && self.bridge_outcome.is_graceful()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeShutdownReport {
    pub outbox_outcome: WorkerOutcome,
    pub bridge_outcome: WorkerOutcome,
    pub jobs_outcome: ports::job_runtime_control::RuntimeShutdownReport,
    pub tracing_outcome: TracingShutdownOutcome,
}

impl RuntimeShutdownReport {
    pub fn is_graceful(&self) -> bool {
        self.outbox_outcome.is_graceful()
            && self.bridge_outcome.is_graceful()
            && self.tracing_outcome.is_graceful()
            && self.jobs_outcome.forced_aborted_count == 0
            && self.jobs_outcome.panicked_count == 0
            && self.jobs_outcome.unconfirmed_count == 0
            && self.jobs_outcome.join_failed_count == 0
    }
}

pub const TRACING_FLUSH_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(500);

pub fn finalize_runtime_shutdown<T: TracingShutdown>(
    workers: WorkerShutdownReport,
    jobs_outcome: ports::job_runtime_control::RuntimeShutdownReport,
    tracing: Option<T>,
) -> RuntimeShutdownReport {
    tracing::info!(
        action = "workers_shutdown_completed",
        outbox_outcome = ?workers.outbox_outcome,
        bridge_outcome = ?workers.bridge_outcome,
        jobs_outcome = ?jobs_outcome,
        "shutdown_handles: workers finished, initiating tracing flush"
    );

    let tracing_outcome = if let Some(guard) = tracing {
        guard.shutdown(TRACING_FLUSH_TIMEOUT)
    } else {
        TracingShutdownOutcome::NotOwned
    };

    RuntimeShutdownReport {
        outbox_outcome: workers.outbox_outcome,
        bridge_outcome: workers.bridge_outcome,
        jobs_outcome,
        tracing_outcome,
    }
}

pub async fn shutdown_runtime(
    outbox: Option<crate::bootstrap::workers::OutboxWorkerHandle>,
    bridge: Option<adapters_tauri::job_event_bridge::JobEventBridgeHandle>,
    timeout: std::time::Duration,
) -> WorkerShutdownReport {
    let deadline = tokio::time::sleep(timeout);
    tokio::pin!(deadline);

    let mut outbox_outcome = WorkerOutcome::AlreadyStopped;
    let mut bridge_outcome = WorkerOutcome::AlreadyStopped;

    let mut outbox_task = None;
    if let Some(handle) = outbox {
        let (tx, task) = handle.into_shutdown_parts();
        if let Some(tx) = tx {
            match tx.try_send(()) {
                Ok(_) | Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {}
                Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                    outbox_outcome = WorkerOutcome::SignalFailed;
                }
            }
        }
        outbox_task = task;
    }

    let mut bridge_task = None;
    if let Some(handle) = bridge {
        let (tx, task) = handle.into_shutdown_parts();
        if let Some(tx) = tx
            && tx.send(()).is_err()
        {
            bridge_outcome = WorkerOutcome::SignalFailed;
        }
        bridge_task = task;
    }

    let mut outbox_done = outbox_task.is_none();
    let mut bridge_done = bridge_task.is_none();

    while !outbox_done || !bridge_done {
        tokio::select! {
            _ = &mut deadline => {
                tracing::warn!(
                    action = "shutdown_timeout",
                    "shutdown_runtime: global deadline reached, aborting remaining tasks"
                );
                if let Some(ref task) = outbox_task {
                    task.abort();
                }
                if let Some(ref task) = bridge_task {
                    task.abort();
                }
                break;
            }
            res = async {
                match &mut outbox_task {
                    Some(t) => t.await,
                    None => std::future::pending().await,
                }
            }, if !outbox_done => {
                outbox_done = true;
                update_outcome(&mut outbox_outcome, res);
                outbox_task = None;
            }
            res = async {
                match &mut bridge_task {
                    Some(t) => t.await,
                    None => std::future::pending().await,
                }
            }, if !bridge_done => {
                bridge_done = true;
                update_outcome(&mut bridge_outcome, res);
                bridge_task = None;
            }
        }
    }

    if let Some(task) = outbox_task {
        update_outcome(&mut outbox_outcome, task.await);
    }
    if let Some(task) = bridge_task {
        update_outcome(&mut bridge_outcome, task.await);
    }

    WorkerShutdownReport {
        outbox_outcome,
        bridge_outcome,
    }
}

fn update_outcome(current: &mut WorkerOutcome, task_result: Result<(), tokio::task::JoinError>) {
    if matches!(current, WorkerOutcome::SignalFailed) {
        return;
    }
    match task_result {
        Ok(_) => *current = WorkerOutcome::Graceful,
        Err(error) if error.is_cancelled() => *current = WorkerOutcome::Aborted,
        Err(_) => *current = WorkerOutcome::JoinFailed,
    }
}
