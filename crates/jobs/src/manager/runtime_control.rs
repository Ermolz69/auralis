use super::manager::JobManager;

#[async_trait::async_trait]
impl ports::job_runtime_control::JobRuntimeControlPort for JobManager {
    async fn reserve(
        &self,
        job_id: domain::job::JobId,
        project_id: domain::project::ProjectId,
    ) -> Result<(), ports::error::PortError> {
        let mut inner = self.runtime_registry.lock_inner();
        if inner.closed {
            return Err(ports::error::PortError::Unexpected {
                message: "Admission closed/shutting down".to_string(),
            });
        }
        if inner.entries.contains_key(&job_id) {
            return Err(ports::error::PortError::Conflict {
                resource: format!("JobRuntimeEntry {}", job_id),
                message: "Job is already registered in the runtime".to_string(),
            });
        }
        inner.entries.insert(
            job_id,
            super::runtime_registry::JobRuntimeEntry::Reserved { project_id },
        );
        Ok(())
    }

    async fn attach_task(
        &self,
        job_id: domain::job::JobId,
        task: ports::job_runtime_control::RuntimeTask,
    ) -> Result<(), ports::job_runtime_control::AttachTaskError> {
        let mut inner = self.runtime_registry.lock_inner();
        if inner.closed {
            return Err(ports::job_runtime_control::AttachTaskError {
                source: ports::error::PortError::Unexpected {
                    message: "Admission closed/shutting down".to_string(),
                },
                task,
            });
        }
        match inner.entries.remove(&job_id) {
            Some(super::runtime_registry::JobRuntimeEntry::Reserved { project_id }) => {
                if task.cancel.is_cancelled() {
                    return Err(ports::job_runtime_control::AttachTaskError {
                        source: ports::error::PortError::Conflict {
                            resource: format!("JobRuntimeEntry {}", job_id),
                            message: "Task is already cancelled before attach".to_string(),
                        },
                        task,
                    });
                }
                inner.entries.insert(
                    job_id,
                    super::runtime_registry::JobRuntimeEntry::Attached { project_id, task },
                );
                Ok(())
            }
            Some(other) => {
                inner.entries.insert(job_id.clone(), other);
                Err(ports::job_runtime_control::AttachTaskError {
                    source: ports::error::PortError::Conflict {
                        resource: format!("JobRuntimeEntry {}", job_id),
                        message: "Job is already attached".to_string(),
                    },
                    task,
                })
            }
            None => Err(ports::job_runtime_control::AttachTaskError {
                source: ports::error::PortError::NotFound {
                    resource: format!("JobRuntimeEntry {}", job_id),
                },
                task,
            }),
        }
    }

    fn finish_now(&self, job_id: &domain::job::JobId) {
        self.runtime_registry.lock_entries().entries.remove(job_id);
    }

    async fn rollback_runtime_start(
        &self,
        job_id: &domain::job::JobId,
    ) -> Result<ports::job_runtime_control::RuntimeCleanupOutcome, ports::error::PortError> {
        let entry_opt = self.runtime_registry.lock_entries().entries.remove(job_id);

        match entry_opt {
            Some(super::runtime_registry::JobRuntimeEntry::Attached { task, .. }) => {
                task.cancel.cancel();
                task.join_handle.abort();
                let _ = task.join_handle.await;
                Ok(ports::job_runtime_control::RuntimeCleanupOutcome::ForcedAborted)
            }
            Some(super::runtime_registry::JobRuntimeEntry::Reserved { .. }) => {
                Ok(ports::job_runtime_control::RuntimeCleanupOutcome::ReservationRemoved)
            }
            None => Ok(ports::job_runtime_control::RuntimeCleanupOutcome::Missing),
        }
    }

    async fn cancel_and_evict_jobs(
        &self,
        job_ids: &[domain::job::JobId],
    ) -> Result<ports::job_runtime_control::RuntimeCleanupReport, ports::error::PortError> {
        self.cancel_and_evict_jobs_internal(job_ids).await
    }

    async fn drain_all(
        &self,
        deadline: std::time::Duration,
    ) -> Result<ports::job_runtime_control::RuntimeShutdownReport, ports::error::PortError> {
        self.drain_all_internal(deadline).await
    }
}
