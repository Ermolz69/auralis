use std::sync::Arc;

use domain::job::JobStatus;
use ports::{
    PortError,
    cancellation::CancelHandle,
    job_execution::{ExecutionContext, JobExecutorPort, QueuedExecution},
    job_runtime_control::{RuntimeCompletion, RuntimeTask, RuntimeTaskOutcome},
    job_scheduler::{JobLifecycleEventKind, JobSchedulerPort},
};

use super::{cpu::JobCpuExecutor, resources::ResourceLease, state::QueueState};
use crate::manager::{JobManager, runtime_registry::JobRuntimeEntry};

pub(super) async fn start(
    manager: &JobManager,
    queue: &Arc<QueueState>,
    executor: Arc<dyn JobExecutorPort>,
    entry: QueuedExecution,
    lease: Arc<ResourceLease>,
) -> Result<bool, PortError> {
    let job_id = entry.job.id().clone();
    let lock = manager.mutation_locks.get_lock(&job_id)?;
    let result = async {
        let _guard = lock.lock().await;
        let Some(job) = manager.repo.get(&job_id).await? else { return Ok(false); };
        if job.status() != &JobStatus::Pending { return Ok(false); }
        let (cancel, token) = CancelHandle::new();
        let completion = Arc::new(RuntimeCompletion::new());
        let (activate, activated) = tokio::sync::oneshot::channel();
        let (ack, acknowledged) = tokio::sync::oneshot::channel();
        let (release, released) = tokio::sync::oneshot::channel();
        {
            let mut registry = manager.runtime_registry.lock_entries();
            if registry.closed { return Err(PortError::AdmissionClosed); }
            if registry.entries.contains_key(&job_id) {
                return Err(PortError::Conflict { resource: format!("Job {job_id}"), message: "Executor already attached".into() });
            }
            let cpu = JobCpuExecutor::new(queue.cpu.clone(), lease.clone(), entry.descriptor.resources.cpu > 0);
            let manager_for_task = manager.clone();
            let queue_for_task = queue.clone();
            let completion_for_task = completion.clone();
            let job_id_for_task = job_id.clone();
            let project_id = job.project_id().clone();
            let may_own_processes = entry.descriptor.resources.processes > 0;
            let resource_guard = lease.clone();
            let task = tokio::spawn(async move {
                let outcome = if activated.await.is_err() || ack.send(()).is_err() || released.await.is_err() || token.is_cancelled() {
                    RuntimeTaskOutcome::Cancelled
                } else {
                    let context = ExecutionContext { cancellation: token, cpu: cpu.clone(), max_subtitle_bytes: queue_for_task.config.max_subtitle_bytes };
                    match futures::FutureExt::catch_unwind(std::panic::AssertUnwindSafe(executor.execute(entry, context))).await {
                        Ok(outcome) => outcome,
                        Err(_) if may_own_processes => RuntimeTaskOutcome::CleanupUnconfirmed,
                        Err(_) => {
                            let result = manager_for_task.fail_job(&job_id_for_task, "RUNNER_PANIC".into(), "Executor panicked".into(), false).await;
                            if result.is_ok() { RuntimeTaskOutcome::Panicked } else { RuntimeTaskOutcome::RecoveryRequired }
                        }
                    }
                };
                // A dropped CPU future is not proof that the synchronous closure stopped.
                cpu.drain().await;
                drop(cpu);
                if outcome == RuntimeTaskOutcome::CleanupUnconfirmed {
                    if let Err(error) = manager_for_task.mutate_job(&job_id_for_task, JobLifecycleEventKind::Cancelling, |job| job.record_unconfirmed_cleanup()).await {
                        tracing::error!(error = %error, "unconfirmed cleanup could not be recorded");
                    }
                    queue_for_task.quarantine.lock().unwrap_or_else(std::sync::PoisonError::into_inner).insert(job_id_for_task.clone(), lease);
                } else {
                    drop(lease);
                    manager_for_task.runtime_registry.lock_entries().entries.remove(&job_id_for_task);
                    manager_for_task.cache.remove(&job_id_for_task).await;
                }
                completion_for_task.finish(outcome);
                queue_for_task.wake.notify_one();
                outcome
            });
            registry.entries.insert(job_id.clone(), JobRuntimeEntry::Managed { project_id, resource_guard, task: RuntimeTask { cancel, join_handle: task, completion: completion.clone() } });
        }
        let attached = activate.send(()).is_ok()
            && matches!(tokio::time::timeout(queue.config.attach_timeout, acknowledged).await, Ok(Ok(())));
        if !attached {
            drop(release);
            return Err(PortError::Unexpected { message: "Executor attach acknowledgement failed".into() });
        }
        let mut running = job.clone();
        running.start().map_err(|error| PortError::Unexpected { message: error.to_string() })?;
        if let Err(error) = queue.store.start(&running).await {
            drop(release);
            return Err(error);
        }
        manager.cache.track_persisted(&running).await;
        manager.emit_job_event(&running, JobLifecycleEventKind::Started);
        if release.send(()).is_err() {
            return Err(PortError::Unexpected { message: "Executor release failed".into() });
        }
        Ok(true)
    }.await;
    let result = manager.finish_mutation(&job_id, &lock, result);
    if let Err(error) = result {
        // Gated tasks have done no useful work and can stop without async cleanup.
        let completion = {
            let registry = manager.runtime_registry.lock_entries();
            match registry.entries.get(&job_id) {
                Some(JobRuntimeEntry::Managed { task, .. }) => {
                    task.cancel.cancel();
                    Some(task.completion.clone())
                }
                _ => None,
            }
        };
        if let Some(completion) = completion {
            completion.wait().await;
        }
        if !matches!(
            error,
            PortError::Conflict { .. } | PortError::NotFound { .. } | PortError::AdmissionClosed
        ) {
            manager
                .fail_job(
                    &job_id,
                    "EXECUTOR_START_FAILED".into(),
                    error.to_string(),
                    false,
                )
                .await?;
        }
        return Err(error);
    }
    result
}
