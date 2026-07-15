use std::str::FromStr;
use std::sync::Arc;

use domain::job::{Job, JobId as DomainJobId, JobKind};
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
    pub(super) mutation_locks: super::mutation_locks::JobMutationLocks,
    pub(super) storage_uow: Arc<dyn ports::transaction::StorageUnitOfWork>,
    pub(super) emitter: Option<JobEventEmitter>,
}

impl JobManager {
    pub fn new(
        repo: Arc<dyn JobRepository>,
        storage_uow: Arc<dyn ports::transaction::StorageUnitOfWork>,
        emitter: Option<JobEventEmitter>,
    ) -> Self {
        Self {
            cache: JobCache::new(),
            repo,
            cancellation_registry: CancellationRegistry::new(),
            mutation_locks: super::mutation_locks::JobMutationLocks::new(),
            storage_uow,
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

    pub(super) async fn mutate_job<F>(
        &self,
        job_id: &DomainJobId,
        action: F,
    ) -> Result<Job, PortError>
    where
        F: FnOnce(&mut Job) -> Result<(), domain::error::DomainError>,
    {
        let lock = self.mutation_locks.get_lock(job_id);
        let _guard = lock.lock().await;

        let mut job = self
            .repo
            .get(job_id)
            .await?
            .ok_or_else(|| PortError::NotFound {
                resource: format!("Job {}", job_id),
            })?;

        action(&mut job).map_err(|e| PortError::Unexpected {
            message: e.to_string(),
        })?;

        self.repo.save(&job).await?;
        self.cache.insert(job.clone()).await;
        self.emit_job_event(&job);

        Ok(job)
    }

    pub(super) async fn mutate_job_terminal<F>(
        &self,
        job_id: &DomainJobId,
        outcome: domain::job::TerminalOutcome,
        action: F,
    ) -> Result<Job, PortError>
    where
        F: FnOnce(&mut Job) -> Result<(), domain::error::DomainError>,
    {
        let lock = self.mutation_locks.get_lock(job_id);
        let _guard = lock.lock().await;

        let mut job = self
            .repo
            .get(job_id)
            .await?
            .ok_or_else(|| PortError::NotFound {
                resource: format!("Job {}", job_id),
            })?;

        action(&mut job).map_err(|e| PortError::Unexpected {
            message: e.to_string(),
        })?;

        let deduplication_key = format!("terminal-{}-{:?}", job.id(), outcome);

        let command = ports::transaction::CommitTerminalJobUpdate {
            job: job.clone(),
            deduplication_key,
            project_id: job.project_id().clone(),
            outcome,
        };

        self.storage_uow.commit_terminal_job_update(command).await?;

        self.cache.insert(job.clone()).await;
        self.emit_job_event(&job);

        Ok(job)
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

#[async_trait::async_trait]
impl ports::job_runtime_control::JobRuntimeControlPort for JobManager {
    async fn cancel_and_evict_jobs(
        &self,
        job_ids: &[domain::job::JobId],
    ) -> Result<(), ports::error::PortError> {
        for job_id in job_ids {
            self.cancellation_registry.cancel_and_wait(job_id).await;

            let lock = self.mutation_locks.get_lock(job_id);
            let _guard = lock.lock().await;

            self.cache.remove(job_id).await;
            self.cancellation_registry.unregister(job_id).await;

            drop(_guard);
            self.mutation_locks.remove(job_id);
        }
        Ok(())
    }
}
