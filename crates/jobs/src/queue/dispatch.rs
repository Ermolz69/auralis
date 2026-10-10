use std::{
    collections::HashMap,
    sync::{Arc, atomic::Ordering},
};

use domain::job::JobId;
use ports::{
    job_execution::{JobExecutorPort, QueuedExecution},
    job_scheduler::JobSchedulerPort,
};

use super::state::QueueState;
use crate::manager::JobManager;

pub(super) async fn run(
    manager: JobManager,
    queue: Arc<QueueState>,
    executor: Arc<dyn JobExecutorPort>,
) {
    let mut bypasses: HashMap<JobId, usize> = HashMap::new();
    loop {
        if queue.closed.load(Ordering::Acquire) {
            break;
        }
        let notified = queue.wake.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        let result = dispatch_window(&manager, &queue, &executor, &mut bypasses).await;
        if queue.closed.load(Ordering::Acquire) {
            break;
        }
        match result {
            Err(error) => {
                tracing::error!(error = %error, "job dispatch requires retry");
                tokio::select! {
                    _ = &mut notified => {},
                    _ = tokio::time::sleep(queue.config.storage_retry_interval) => {},
                }
            }
            Ok(false) => {
                // Durable polling repairs a wake lost when an admission future is dropped at commit.
                tokio::select! {
                    _ = &mut notified => {},
                    _ = tokio::time::sleep(queue.config.storage_retry_interval) => {},
                }
            }
            Ok(true) => {}
        }
    }
}

async fn dispatch_window(
    manager: &JobManager,
    queue: &Arc<QueueState>,
    executor: &Arc<dyn JobExecutorPort>,
    bypasses: &mut HashMap<JobId, usize>,
) -> Result<bool, ports::PortError> {
    let _guard = queue.gate.lock().await;
    if queue.closed.load(Ordering::Acquire) {
        return Ok(false);
    }
    let window = queue
        .store
        .list_pending(queue.config.dispatch_window)
        .await?;
    bypasses.retain(|id, _| window.iter().any(|item| item.job.id() == id));
    let mut blocked: Vec<QueuedExecution> = Vec::new();
    let mut started = false;
    for entry in window {
        if queue.closed.load(Ordering::Acquire) {
            break;
        }
        let validated = entry.descriptor.and_then(|descriptor| {
            descriptor.validate()?;
            queue.config.validate_demand(descriptor.resources)?;
            executor.validate(&descriptor)?;
            if descriptor.project_id != *entry.job.project_id() {
                return Err(ports::PortError::InvalidSource {
                    message: "Execution project does not match the job".into(),
                });
            }
            Ok(descriptor)
        });
        let descriptor = match validated {
            Ok(descriptor) => descriptor,
            Err(error) => {
                manager
                    .fail_job(
                        entry.job.id(),
                        "EXECUTION_DESCRIPTOR_INVALID".into(),
                        error.to_string(),
                        false,
                    )
                    .await?;
                started = true;
                continue;
            }
        };
        let entry = QueuedExecution {
            job: entry.job,
            descriptor,
        };
        let demand = entry.descriptor.resources;
        let fenced = blocked.iter().any(|older| {
            bypasses.get(older.job.id()).copied().unwrap_or(0) >= queue.config.max_bypasses
                && older.descriptor.resources.overlaps(demand)
        });
        let lease = if fenced {
            None
        } else {
            queue.resources.reserve(demand)
        };
        if let Some(lease) = lease {
            match super::worker::start(manager, queue, executor.clone(), entry, lease).await {
                Ok(true) => {
                    for older in &blocked {
                        *bypasses.entry(older.job.id().clone()).or_default() += 1;
                    }
                    started = true;
                }
                Ok(false) => {
                    started = true;
                }
                Err(error) => return Err(error),
            }
        } else {
            blocked.push(entry);
        }
    }
    Ok(started)
}
