use crate::error::ApplicationError;
use ports::job_scheduler::{JobSchedulerPort, ScheduledJob, StartDubbingJobRequest};
use std::sync::Arc;

pub struct StartMockJobRequest {
    pub title: String,
}

pub struct StartMockJobResponse {
    pub job: ScheduledJob,
}

pub struct StartMockJobUseCase {
    scheduler: Arc<dyn JobSchedulerPort>,
}

impl StartMockJobUseCase {
    pub fn new(scheduler: Arc<dyn JobSchedulerPort>) -> Self {
        Self { scheduler }
    }

    pub async fn execute(
        &self,
        req: StartMockJobRequest,
    ) -> Result<StartMockJobResponse, ApplicationError> {
        let job = self
            .scheduler
            .start_dubbing_job(StartDubbingJobRequest {
                title: req.title,
                project_id: None,
            })
            .await?;

        Ok(StartMockJobResponse { job })
    }
}
