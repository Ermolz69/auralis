use crate::error::ApplicationError;
use domain::job::JobId;
use ports::job_scheduler::{JobSchedulerPort, ScheduledJob};
use std::sync::Arc;

pub struct CancelJobRequest {
    pub job_id: JobId,
}

pub struct CancelJobResponse {
    pub job: ScheduledJob,
}

pub struct CancelJobUseCase {
    scheduler: Arc<dyn JobSchedulerPort>,
}

impl CancelJobUseCase {
    pub fn new(scheduler: Arc<dyn JobSchedulerPort>) -> Self {
        Self { scheduler }
    }

    pub async fn execute(
        &self,
        req: CancelJobRequest,
    ) -> Result<CancelJobResponse, ApplicationError> {
        let job = self.scheduler.cancel_job(&req.job_id).await?;
        Ok(CancelJobResponse { job })
    }
}
