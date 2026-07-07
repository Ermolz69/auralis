use async_trait::async_trait;
use chrono::Utc;
use domain::job::JobId;
use ports::error::PortError;
use ports::job_scheduler::{JobSchedulerPort, ScheduledJob, StartDubbingJobRequest};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct MockJobScheduler {
    pub jobs: Arc<Mutex<Vec<ScheduledJob>>>,
}

impl MockJobScheduler {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl JobSchedulerPort for MockJobScheduler {
    async fn start_dubbing_job(
        &self,
        request: StartDubbingJobRequest,
    ) -> Result<ScheduledJob, PortError> {
        let job = ScheduledJob {
            id: JobId::new(),
            project_id: request.project_id,
            title: request.title,
            status: domain::job::JobStatus::Pending,
            stage: None,
            progress: domain::job::JobProgress::initializing(),
            error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mut jobs = self.jobs.lock().await;
        jobs.push(job.clone());

        Ok(job)
    }

    async fn cancel_job(&self, job_id: &JobId) -> Result<ScheduledJob, PortError> {
        let mut jobs = self.jobs.lock().await;
        if let Some(job) = jobs.iter_mut().find(|j| j.id == *job_id) {
            job.status = domain::job::JobStatus::Cancelled;
            job.updated_at = Utc::now();
            return Ok(job.clone());
        }
        Err(PortError::NotFound {
            resource: format!("Job {}", job_id),
        })
    }

    async fn get_job(&self, job_id: &JobId) -> Result<Option<ScheduledJob>, PortError> {
        let jobs = self.jobs.lock().await;
        Ok(jobs.iter().find(|j| j.id == *job_id).cloned())
    }

    async fn list_jobs(&self) -> Result<Vec<ScheduledJob>, PortError> {
        let jobs = self.jobs.lock().await;
        Ok(jobs.clone())
    }
}
