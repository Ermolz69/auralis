use crate::error::ApplicationError;
use ports::job_scheduler::{JobSchedulerPort, ScheduledJob};
use std::sync::Arc;

pub struct ListJobsRequest {}

pub struct ListJobsResponse {
    pub jobs: Vec<ScheduledJob>,
}

pub struct ListJobsUseCase {
    scheduler: Arc<dyn JobSchedulerPort>,
}

impl ListJobsUseCase {
    pub fn new(scheduler: Arc<dyn JobSchedulerPort>) -> Self {
        Self { scheduler }
    }

    pub async fn execute(
        &self,
        _req: ListJobsRequest,
    ) -> Result<ListJobsResponse, ApplicationError> {
        let jobs = self.scheduler.list_jobs().await?;
        Ok(ListJobsResponse { jobs })
    }
}
