use super::manager::JobManager;
use super::mapper::map_job_to_scheduled;
use domain::job::{Job, JobId as DomainJobId};
use ports::job_scheduler::JobLifecycleEvent;

impl JobManager {
    pub(super) async fn cancel_runtime_and_wait(
        &self,
        job_id: &DomainJobId,
    ) -> Option<ports::job_runtime_control::RuntimeTaskOutcome> {
        let completion = {
            let mut registry = self.runtime_registry.lock_entries();
            match registry.entries.get(job_id) {
                Some(super::runtime_registry::JobRuntimeEntry::Attached { task, .. }) => {
                    task.cancel.cancel();
                    Some((task.completion.clone(), false))
                }
                Some(super::runtime_registry::JobRuntimeEntry::Managed { task, .. }) => {
                    task.cancel.cancel();
                    Some((task.completion.clone(), true))
                }
                Some(super::runtime_registry::JobRuntimeEntry::Reserved { .. }) => {
                    registry.entries.remove(job_id);
                    None
                }
                None => None,
            }
        };

        match completion {
            Some((completion, true)) => {
                let timeout = self
                    .queue
                    .as_ref()
                    .map_or(std::time::Duration::from_secs(7), |queue| {
                        queue.config.cleanup_timeout
                    });
                Some(
                    tokio::time::timeout(timeout, completion.wait())
                        .await
                        .unwrap_or(
                            ports::job_runtime_control::RuntimeTaskOutcome::CleanupUnconfirmed,
                        ),
                )
            }
            Some((completion, false)) => Some(completion.wait().await),
            None => None,
        }
    }

    pub(crate) fn emit_job_event(
        &self,
        job: &Job,
        kind: ports::job_scheduler::JobLifecycleEventKind,
    ) {
        let scheduled = map_job_to_scheduled(job);

        let event = JobLifecycleEvent {
            kind,
            job: scheduled,
        };

        if let Some(emitter) = &self.emitter
            && std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| emitter(event))).is_err()
        {
            tracing::error!("job event subscriber panicked after durable persistence");
        }
    }
}
