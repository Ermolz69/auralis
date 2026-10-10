use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use domain::job::JobId;
use ports::{
    PortError,
    job_execution::{JobExecutionStore, JobExecutorPort},
};
use tokio::sync::{Mutex, Notify};

use super::{
    JobQueueConfig,
    cpu::CpuPool,
    resources::{ResourceLease, ResourcePool},
};
use crate::manager::JobManager;

pub(crate) struct QueueState {
    pub config: JobQueueConfig,
    pub store: Arc<dyn JobExecutionStore>,
    pub resources: Arc<ResourcePool>,
    pub cpu: Arc<CpuPool>,
    pub wake: Arc<Notify>,
    pub closed: AtomicBool,
    pub gate: Mutex<()>,
    pub handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    pub executor: std::sync::Mutex<Option<Arc<dyn JobExecutorPort>>>,
    pub quarantine: std::sync::Mutex<HashMap<JobId, Arc<ResourceLease>>>,
}

impl JobManager {
    pub fn with_queue(
        mut self,
        store: Arc<dyn JobExecutionStore>,
        config: JobQueueConfig,
    ) -> Result<Self, PortError> {
        config.validate()?;
        let wake = Arc::new(Notify::new());
        self.queue = Some(Arc::new(QueueState {
            resources: ResourcePool::new(config.budgets, config.max_active, wake.clone()),
            cpu: CpuPool::new(config.budgets.cpu, config.cpu_backlog)?,
            config,
            store,
            wake,
            closed: AtomicBool::new(false),
            gate: Mutex::new(()),
            handle: Mutex::new(None),
            executor: std::sync::Mutex::new(None),
            quarantine: std::sync::Mutex::new(HashMap::new()),
        }));
        Ok(self)
    }

    pub async fn start_dispatcher(
        &self,
        executor: Arc<dyn JobExecutorPort>,
    ) -> Result<(), PortError> {
        let queue = self.queue.as_ref().ok_or_else(|| PortError::Unsupported {
            message: "Job queue is not configured".into(),
        })?;
        let _guard = queue.gate.lock().await;
        if queue.closed.load(Ordering::Acquire) {
            return Err(PortError::AdmissionClosed);
        }
        {
            let mut current = queue
                .executor
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if current.is_some() {
                return Err(PortError::Conflict {
                    resource: "Job dispatcher".into(),
                    message: "Already started".into(),
                });
            }
            *current = Some(executor.clone());
        }
        let manager = self.clone();
        let queue_for_task = queue.clone();
        *queue.handle.lock().await = Some(tokio::spawn(async move {
            super::dispatch::run(manager, queue_for_task, executor).await;
        }));
        Ok(())
    }

    pub fn queue_available_resources(&self) -> Option<ports::job_resources::ResourceDemand> {
        self.queue.as_ref().map(|q| q.resources.available())
    }

    pub fn queue_config(&self) -> Option<&JobQueueConfig> {
        self.queue.as_ref().map(|q| &q.config)
    }

    pub(crate) async fn stop_dispatcher(&self, deadline: tokio::time::Instant) -> bool {
        let Some(queue) = &self.queue else {
            return true;
        };
        queue.closed.store(true, Ordering::Release);
        queue
            .executor
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        queue.wake.notify_one();
        if let Some(mut handle) = queue.handle.lock().await.take()
            && tokio::time::timeout_at(deadline, &mut handle)
                .await
                .is_err()
        {
            handle.abort();
            let _ = handle.await;
            return false;
        }
        true
    }
}
