use std::sync::atomic::Ordering;

use ports::{
    PortError,
    job_execution::{ExecutionDescriptor, JobQueuePort},
    job_scheduler::{JobLifecycleEventKind, ScheduledJob},
    transaction::CommitPipelineStart,
};

use crate::manager::{JobManager, mapper::map_job_to_scheduled};

#[async_trait::async_trait]
impl JobQueuePort for JobManager {
    async fn admit_pipeline(
        &self,
        start: CommitPipelineStart,
        descriptor: ExecutionDescriptor,
    ) -> Result<ScheduledJob, PortError> {
        let queue = self.queue.as_ref().ok_or_else(|| PortError::Unsupported {
            message: "Job queue is not configured".into(),
        })?;
        let _guard = queue.gate.lock().await;
        if queue.closed.load(Ordering::Acquire) {
            return Err(PortError::AdmissionClosed);
        }
        start.validate()?;
        descriptor.validate()?;
        queue.config.validate_demand(descriptor.resources)?;
        let executor = queue
            .executor
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .ok_or(PortError::AdmissionClosed)?;
        executor.validate(&descriptor)?;
        if descriptor.project_id != *start.project.id() {
            return Err(PortError::InvalidSource {
                message: "Execution belongs to another project".into(),
            });
        }
        let job = start.job.clone();
        queue
            .store
            .admit_pipeline(start, descriptor, queue.config.pending_capacity)
            .await?;
        self.emit_job_event(&job, JobLifecycleEventKind::Created);
        queue.wake.notify_one();
        Ok(map_job_to_scheduled(&job))
    }
}
