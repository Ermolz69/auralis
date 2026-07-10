use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;

use domain::job::{Job, JobId as DomainJobId, JobKind, JobStatus};
use domain::project::ProjectId as DomainProjectId;

use async_trait::async_trait;
use ports::job_scheduler::JobLifecycleEvent;

use ports::error::PortError;
use ports::job_scheduler::{JobSchedulerPort, ScheduledJob, StartDubbingJobRequest};
use ports::repository::JobRepository;

pub type JobEventEmitter = Arc<dyn Fn(JobLifecycleEvent) + Send + Sync + 'static>;

#[derive(Clone)]
pub struct JobManager {
    jobs: Arc<RwLock<HashMap<DomainJobId, Job>>>,
    repo: Arc<dyn JobRepository>,
    cancel_handles: Arc<RwLock<HashMap<DomainJobId, crate::cancellation::CancelHandle>>>,
    emitter: Option<JobEventEmitter>,
}

impl JobManager {
    pub fn new(repo: Arc<dyn JobRepository>, emitter: Option<JobEventEmitter>) -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            repo,
            cancel_handles: Arc::new(RwLock::new(HashMap::new())),
            emitter,
        }
    }

    pub async fn load_recent_jobs(&self, limit: usize) -> Result<(), PortError> {
        let recent = self.repo.list_recent(limit).await?;
        let mut jobs = self.jobs.write().await;
        for job in recent {
            jobs.insert(job.id().clone(), job);
        }
        Ok(())
    }

    pub async fn start_mock_dubbing_job_internal(
        &self,
        title: String,
        project_id_str: Option<String>,
    ) -> Result<Job, PortError> {
        let id_str = project_id_str.ok_or_else(|| PortError::Unexpected {
            message: "Missing project_id for mock dubbing job".to_string(),
        })?;

        let project_id = DomainProjectId::from_str(&id_str).map_err(|e| PortError::Unexpected {
            message: format!("Invalid project_id {}: {}", id_str, e),
        })?;

        let mut job = Job::new(project_id, title, JobKind::Dubbing);
        job.start().map_err(|e| PortError::Unexpected {
            message: e.to_string(),
        })?;

        let job = self.repo.create(job).await?;
        let job_id = job.id().clone();

        self.jobs.write().await.insert(job_id.clone(), job.clone());
        self.emit_job_event(&job);

        Ok(job)
    }

    pub async fn get_job_internal(&self, job_id: &DomainJobId) -> Option<Job> {
        if let Some(job) = self.jobs.read().await.get(job_id) {
            return Some(job.clone());
        }

        if let Ok(Some(job)) = self.repo.get(job_id).await {
            self.jobs.write().await.insert(job_id.clone(), job.clone());
            return Some(job);
        }

        None
    }

    pub async fn list_jobs_internal(&self) -> Vec<Job> {
        let mut jobs = match self.repo.list_recent(100).await {
            Ok(j) => j,
            Err(e) => {
                eprintln!("WARNING: Failed to list recent jobs from repo: {}", e);
                // Fallback to cache
                self.jobs.read().await.values().cloned().collect()
            }
        };

        // Merge active jobs from cache if any aren't returned by list_recent
        let cache = self.jobs.read().await;
        for (id, active_job) in cache.iter() {
            if matches!(
                active_job.status(),
                domain::job::JobStatus::Pending | domain::job::JobStatus::Running
            ) {
                if !jobs.iter().any(|j| j.id() == id) {
                    jobs.push(active_job.clone());
                } else {
                    // Update the repo version with the more up-to-date cache version (e.g. progress updates)
                    if let Some(existing) = jobs.iter_mut().find(|j| j.id() == id) {
                        *existing = active_job.clone();
                    }
                }
            }
        }

        jobs.sort_by_key(|b| std::cmp::Reverse(*b.created_at()));
        jobs
    }

    pub async fn cancel_job_internal(&self, job_id: &DomainJobId) -> Result<Job, PortError> {
        let mut job = self
            .get_job_internal(job_id)
            .await
            .ok_or_else(|| PortError::NotFound {
                resource: format!("Job {}", job_id),
            })?;

        let should_cancel = matches!(job.status(), JobStatus::Pending | JobStatus::Running);
        if should_cancel {
            job.cancel().ok();

            {
                let handles = self.cancel_handles.read().await;
                if let Some(handle) = handles.get(job_id) {
                    handle.cancel();
                }
            }
            self.update_job(job.clone()).await?;
        }

        Ok(job)
    }

    pub async fn update_job(&self, updated_job: Job) -> Result<(), PortError> {
        self.repo.save(&updated_job).await?;
        {
            let mut jobs = self.jobs.write().await;
            jobs.insert(updated_job.id().clone(), updated_job.clone());
        }
        self.emit_job_event(&updated_job);
        Ok(())
    }

    pub async fn register_cancel_handle(
        &self,
        id: DomainJobId,
        handle: crate::cancellation::CancelHandle,
    ) {
        let mut handles = self.cancel_handles.write().await;
        handles.insert(id, handle);
    }

    pub async fn remove_cancel_handle(&self, id: &DomainJobId) {
        let mut handles = self.cancel_handles.write().await;
        handles.remove(id);
    }

    fn map_job_to_scheduled(job: &Job) -> ScheduledJob {
        ScheduledJob {
            id: job.id().clone(),
            project_id: Some(job.project_id().clone()),
            title: job.title().to_string(),
            status: job.status().clone(),
            stage: job.stage().cloned(),
            progress: job.progress().clone(),
            error: job.error().map(|e| e.message.clone()),
            created_at: *job.created_at(),
            updated_at: *job.updated_at(),
        }
    }

    fn emit_job_event(&self, job: &Job) {
        let scheduled = Self::map_job_to_scheduled(job);

        let event = JobLifecycleEvent {
            job_id: scheduled.id,
            project_id: scheduled.project_id,
            status: scheduled.status,
            stage: scheduled.stage,
            progress: scheduled.progress,
            error: scheduled.error,
        };

        if let Some(emitter) = &self.emitter {
            emitter(event);
        }
    }
}

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

        Ok(Self::map_job_to_scheduled(&job))
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
                return Ok(Self::map_job_to_scheduled(&job));
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
        self.jobs.write().await.insert(job_id.clone(), job.clone());

        // 6. Emit event
        self.emit_job_event(&job);

        Ok(Self::map_job_to_scheduled(&job))
    }

    async fn cancel_job(&self, job_id: &DomainJobId) -> Result<ScheduledJob, PortError> {
        let job = self.cancel_job_internal(job_id).await?;
        Ok(Self::map_job_to_scheduled(&job))
    }

    async fn get_job(&self, job_id: &DomainJobId) -> Result<Option<ScheduledJob>, PortError> {
        let job = self.get_job_internal(job_id).await;
        Ok(job.map(|j| Self::map_job_to_scheduled(&j)))
    }

    async fn list_jobs(&self) -> Result<Vec<ScheduledJob>, PortError> {
        let jobs = self.list_jobs_internal().await;
        Ok(jobs
            .into_iter()
            .map(|j| Self::map_job_to_scheduled(&j))
            .collect())
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

        Ok(Self::map_job_to_scheduled(&job))
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

        Ok(Self::map_job_to_scheduled(&job))
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

        Ok(Self::map_job_to_scheduled(&job))
    }
}
