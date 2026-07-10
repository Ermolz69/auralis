use async_trait::async_trait;
use domain::job::{JobId as DomainJobId, JobStatus};
use ports::error::PortError;
use ports::job_scheduler::{JobSchedulerPort, ScheduledJob, StartDubbingJobRequest};

use super::manager::JobManager;
use super::mapper::map_job_to_scheduled;

#[async_trait]
impl JobSchedulerPort for JobManager {
    async fn start_dubbing_job(
        &self,
        request: StartDubbingJobRequest,
    ) -> Result<ScheduledJob, PortError> {
        let project_id_str = request.project_id.map(|id| id.to_string());
        let job = self
            .start_mock_dubbing_job_internal(request.title, project_id_str)
            .await?;

        Ok(map_job_to_scheduled(&job))
    }

    async fn enqueue_existing_job(&self, job_id: &DomainJobId) -> Result<ScheduledJob, PortError> {
        // 1. Get from repo
        let mut job = self
            .repo
            .get(job_id)
            .await?
            .ok_or_else(|| PortError::NotFound {
                resource: format!("Job {}", job_id),
            })?;

        // 2. Validate status
        if job.status() != &domain::job::JobStatus::Pending {
            // If it's already running, we can just return it idly. If it's something else, return error.
            if job.status() == &domain::job::JobStatus::Running {
                return Ok(map_job_to_scheduled(&job));
            }
            return Err(PortError::Unexpected {
                message: format!(
                    "Cannot enqueue job {} with status {:?}",
                    job_id,
                    job.status()
                ),
            });
        }

        // 3. Start the job
        job.start().map_err(|e| PortError::Unexpected {
            message: e.to_string(),
        })?;

        // 4. Save
        self.repo.save(&job).await?;

        // 5. Update in memory map
        self.cache.insert(job.clone()).await;

        // 6. Emit event
        self.emit_job_event(&job);

        Ok(map_job_to_scheduled(&job))
    }

    async fn cancel_job(&self, job_id: &DomainJobId) -> Result<ScheduledJob, PortError> {
        let job = self.cancel_job_internal(job_id).await?;
        Ok(map_job_to_scheduled(&job))
    }

    async fn get_job(&self, job_id: &DomainJobId) -> Result<Option<ScheduledJob>, PortError> {
        let job = self.get_job_internal(job_id).await;
        Ok(job.map(|j| map_job_to_scheduled(&j)))
    }

    async fn list_jobs(&self) -> Result<Vec<ScheduledJob>, PortError> {
        let jobs = self.list_jobs_internal().await;
        Ok(jobs.into_iter().map(|j| map_job_to_scheduled(&j)).collect())
    }

    async fn update_job_stage(
        &self,
        job_id: &DomainJobId,
        stage: domain::dubbing::DubbingPipelineStage,
        progress: domain::job::JobProgress,
    ) -> Result<ScheduledJob, PortError> {
        let mut job = self
            .get_job_internal(job_id)
            .await
            .ok_or_else(|| PortError::NotFound {
                resource: format!("Job {}", job_id),
            })?;

        if job.status() != &JobStatus::Running {
            return Err(PortError::Unexpected {
                message: format!("Cannot update stage for job in status {:?}", job.status()),
            });
        }

        job.update_stage(stage).ok();
        job.update_progress(progress).ok();

        self.update_job(job.clone()).await?;

        Ok(map_job_to_scheduled(&job))
    }

    async fn complete_job(&self, job_id: &DomainJobId) -> Result<ScheduledJob, PortError> {
        let mut job = self
            .get_job_internal(job_id)
            .await
            .ok_or_else(|| PortError::NotFound {
                resource: format!("Job {}", job_id),
            })?;

        job.mark_completed().map_err(|e| PortError::Unexpected {
            message: e.to_string(),
        })?;

        self.update_job(job.clone()).await?;

        Ok(map_job_to_scheduled(&job))
    }

    async fn fail_job(
        &self,
        job_id: &DomainJobId,
        code: String,
        message: String,
        _retryable: bool,
    ) -> Result<ScheduledJob, PortError> {
        let mut job = self
            .get_job_internal(job_id)
            .await
            .ok_or_else(|| PortError::NotFound {
                resource: format!("Job {}", job_id),
            })?;

        job.mark_failed(domain::job::JobError::new(code, message, _retryable))
            .map_err(|e| PortError::Unexpected {
                message: e.to_string(),
            })?;

        self.update_job(job.clone()).await?;

        Ok(map_job_to_scheduled(&job))
    }
}
