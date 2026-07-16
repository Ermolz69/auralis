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
    pub should_fail: bool,
}

impl Default for MockJobScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl MockJobScheduler {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(Vec::new())),
            should_fail: false,
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
            revision: 0,
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

    async fn enqueue_existing_job(&self, job_id: &JobId) -> Result<ScheduledJob, PortError> {
        if self.should_fail {
            return Err(PortError::Unexpected {
                message: "Mock scheduling failure".to_string(),
            });
        }
        let job = ScheduledJob {
            id: job_id.clone(),
            revision: 0,
            project_id: None,
            title: "Mock Enqueued Job".to_string(),
            status: domain::job::JobStatus::Running,
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

    async fn list_jobs_snapshot(
        &self,
        project_id: &domain::project::ProjectId,
    ) -> Result<Vec<ScheduledJob>, PortError> {
        let jobs = self.jobs.lock().await;
        let filtered = jobs
            .iter()
            .filter(|j| j.project_id.as_ref() == Some(project_id))
            .cloned()
            .collect();
        Ok(filtered)
    }

    async fn update_job_stage(
        &self,
        job_id: &JobId,
        stage: domain::dubbing::DubbingPipelineStage,
        progress: domain::job::JobProgress,
    ) -> Result<ScheduledJob, PortError> {
        let mut jobs = self.jobs.lock().await;
        if let Some(job) = jobs.iter_mut().find(|j| j.id == *job_id) {
            job.stage = Some(stage.clone());
            job.progress = progress;
            job.updated_at = Utc::now();
            return Ok(job.clone());
        }
        Err(PortError::NotFound {
            resource: format!("Job {}", job_id),
        })
    }

    async fn complete_job(&self, job_id: &JobId) -> Result<ScheduledJob, PortError> {
        let mut jobs = self.jobs.lock().await;
        if let Some(job) = jobs.iter_mut().find(|j| j.id == *job_id) {
            job.status = domain::job::JobStatus::Completed;
            job.updated_at = Utc::now();
            return Ok(job.clone());
        }
        Err(PortError::NotFound {
            resource: format!("Job {}", job_id),
        })
    }

    async fn fail_job(
        &self,
        job_id: &JobId,
        _error_code: String,
        error_message: String,
        _retryable: bool,
    ) -> Result<ScheduledJob, PortError> {
        let mut jobs = self.jobs.lock().await;
        if let Some(job) = jobs.iter_mut().find(|j| j.id == *job_id) {
            job.status = domain::job::JobStatus::Failed;
            job.error = Some(error_message);
            job.updated_at = Utc::now();
            return Ok(job.clone());
        }
        Err(PortError::NotFound {
            resource: format!("Job {}", job_id),
        })
    }
}
