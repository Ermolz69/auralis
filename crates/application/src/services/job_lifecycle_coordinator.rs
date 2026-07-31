use crate::error::ApplicationError;
use ports::job_scheduler::JobLifecycleEvent;

#[derive(Default)]
pub struct JobLifecycleCoordinator {}

impl JobLifecycleCoordinator {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn handle(&self, _event: JobLifecycleEvent) -> Result<(), ApplicationError> {
        // terminal job state is handled by outbox worker
        Ok(())
    }
}
