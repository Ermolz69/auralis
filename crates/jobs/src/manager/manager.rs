use std::str::FromStr;
use std::sync::Arc;

use domain::job::{Job, JobId as DomainJobId, JobKind, JobStatus};
use domain::project::ProjectId as DomainProjectId;
use ports::error::PortError;
use ports::job_scheduler::JobLifecycleEvent;
use ports::repository::JobRepository;

use super::cache::JobCache;
use super::cancellation_registry::CancellationRegistry;
use super::mapper::map_job_to_scheduled;

pub type JobEventEmitter = Arc<dyn Fn(JobLifecycleEvent) + Send + Sync + 'static>;

#[derive(Clone)]
pub struct JobManager {
    pub(super) cache: JobCache,
    pub(super) repo: Arc<dyn JobRepository>,
    pub(super) cancellation_registry: CancellationRegistry,
    pub(super) emitter: Option<JobEventEmitter>,
}

impl JobManager {
    pub fn new(repo: Arc<dyn JobRepository>, emitter: Option<JobEventEmitter>) -> Self {
        Self {
            cache: JobCache::new(),
            repo,
            cancellation_registry: CancellationRegistry::new(),
            emitter,
        }
    }

    pub async fn load_recent_jobs(&self, limit: usize) -> Result<(), PortError> {
        let recent = self.repo.list_recent(limit).await?;
        for job in recent {
            self.cache.insert(job).await;
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
        self.cache.insert(job.clone()).await;
        self.emit_job_event(&job);

        Ok(job)
    }

    pub async fn get_job_internal(&self, job_id: &DomainJobId) -> Option<Job> {
        if let Some(job) = self.cache.get(job_id).await {
            return Some(job);
        }

        if let Ok(Some(job)) = self.repo.get(job_id).await {
            self.cache.insert(job.clone()).await;
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
                self.cache.list_all().await
            }
        };

        // Merge active jobs from cache if any aren't returned by list_recent
        let cache_jobs = self.cache.list_all().await;
        for active_job in cache_jobs {
            if matches!(
                active_job.status(),
                domain::job::JobStatus::Pending | domain::job::JobStatus::Running
            ) {
                if !jobs.iter().any(|j| j.id() == active_job.id()) {
                    jobs.push(active_job.clone());
                } else {
                    // Update the repo version with the more up-to-date cache version (e.g. progress updates)
                    if let Some(existing) = jobs.iter_mut().find(|j| j.id() == active_job.id()) {
                        *existing = active_job;
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

            self.cancellation_registry.cancel(job_id).await;
            self.update_job(job.clone()).await?;
        }

        Ok(job)
    }

    pub async fn update_job(&self, updated_job: Job) -> Result<(), PortError> {
        self.repo.save(&updated_job).await?;
        self.cache.insert(updated_job.clone()).await;
        self.emit_job_event(&updated_job);
        Ok(())
    }

    pub async fn register_cancel_handle(
        &self,
        id: DomainJobId,
        handle: crate::cancellation::CancelHandle,
    ) {
        self.cancellation_registry.register(id, handle).await;
    }

    pub async fn remove_cancel_handle(&self, id: &DomainJobId) {
        self.cancellation_registry.unregister(id).await;
    }

    pub(super) fn emit_job_event(&self, job: &Job) {
        let scheduled = map_job_to_scheduled(job);

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
