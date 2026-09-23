use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;

use domain::job::{Job, JobId as DomainJobId, JobKind, JobStatus};
use domain::project::ProjectId as DomainProjectId;
use ports::error::PortError;
use ports::job_scheduler::JobLifecycleEvent;
use ports::repository::JobRepository;

use super::cache::JobCache;
use super::mapper::map_job_to_scheduled;
use super::runtime_registry::RuntimeRegistry;
pub type JobEventEmitter = Arc<dyn Fn(JobLifecycleEvent) + Send + Sync + 'static>;

fn is_active_job(job: &Job) -> bool {
    matches!(
        job.status(),
        JobStatus::Pending | JobStatus::Running | JobStatus::Cancelling
    )
}

#[derive(Clone)]
pub struct JobManager {
    pub(super) cache: JobCache,
    pub(super) repo: Arc<dyn JobRepository>,
    pub(crate) runtime_registry: RuntimeRegistry,
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
            runtime_registry: RuntimeRegistry::new(),
            mutation_locks: super::mutation_locks::JobMutationLocks::new(),
            storage_uow,
            emitter,
        }
    }

    pub async fn load_recent_jobs(&self, limit: usize) -> Result<(), PortError> {
        let recent = self.repo.list_recent(limit).await?;
        for job in recent {
            self.cache.track_persisted(&job).await;
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn mutation_lock_for_test(
        &self,
        job_id: &DomainJobId,
    ) -> Result<Arc<tokio::sync::Mutex<()>>, PortError> {
        self.mutation_locks.get_lock(job_id)
    }

    #[cfg(test)]
    pub(crate) async fn is_job_cached_for_test(&self, job_id: &DomainJobId) -> bool {
        self.cache.contains(job_id).await
    }

    #[cfg(test)]
    pub(crate) async fn active_cache_len_for_test(&self) -> usize {
        self.cache.active_count().await
    }

    #[cfg(test)]
    pub(crate) fn mutation_lock_count_for_test(&self) -> Result<usize, PortError> {
        self.mutation_locks.entry_count()
    }

    pub async fn start_mock_dubbing_job_internal(
        &self,
        title: String,
        project_id_str: Option<String>,
    ) -> Result<Job, PortError> {
        let id_str = project_id_str.ok_or_else(|| PortError::Unexpected {
            message: "Missing project_id for mock dubbing job".to_string(),
        })?;

        let project_id = DomainProjectId::from_str(&id_str).map_err(|_| PortError::Unexpected {
            message: "Invalid project_id for mock dubbing job".to_string(),
        })?;

        let mut job = Job::new(project_id, title, JobKind::Dubbing);
        job.start().map_err(|e| PortError::Unexpected {
            message: e.to_string(),
        })?;

        let job = self.repo.create(job).await?;
        self.cache.track_persisted(&job).await;
        self.emit_job_event(&job, ports::job_scheduler::JobLifecycleEventKind::Created);

        Ok(job)
    }

    pub async fn get_job_internal(&self, job_id: &DomainJobId) -> Result<Option<Job>, PortError> {
        match self.repo.get(job_id).await? {
            Some(job) => {
                if !is_active_job(&job) {
                    self.cache.remove(job_id).await;
                }
                Ok(Some(job))
            }
            None => {
                self.cache.remove(job_id).await;
                Ok(None)
            }
        }
    }

    pub async fn list_jobs_internal(&self) -> Result<Vec<Job>, PortError> {
        let mut jobs = self.repo.list_recent(100).await?;
        let mut listed_ids: HashSet<_> = jobs.iter().map(|job| job.id().clone()).collect();

        for job in &jobs {
            if !is_active_job(job) {
                self.cache.remove(job.id()).await;
            }
        }

        for job_id in self.cache.list_active_ids().await {
            if listed_ids.contains(&job_id) {
                continue;
            }

            match self.repo.get(&job_id).await? {
                Some(job) if is_active_job(&job) => {
                    listed_ids.insert(job_id);
                    jobs.push(job);
                }
                Some(_) | None => self.cache.remove(&job_id).await,
            }
        }

        jobs.sort_by_key(|b| std::cmp::Reverse(*b.created_at()));
        Ok(jobs)
    }

    pub(super) async fn mutate_job<F>(
        &self,
        job_id: &DomainJobId,
        kind: ports::job_scheduler::JobLifecycleEventKind,
        action: F,
    ) -> Result<Job, PortError>
    where
        F: FnOnce(&mut Job) -> Result<(), domain::error::DomainError>,
    {
        let lock = self.mutation_locks.get_lock(job_id)?;
        let result = async {
            let _guard = lock.lock().await;

            let mut job = self
                .repo
                .get(job_id)
                .await?
                .ok_or_else(|| PortError::NotFound {
                    resource: format!("Job {}", job_id),
                })?;

            let expected_revision = job.revision();
            action(&mut job).map_err(|e| PortError::Unexpected {
                message: e.to_string(),
            })?;

            if job.revision() != expected_revision {
                self.repo.save(&job, expected_revision).await?;
                self.cache.track_persisted(&job).await;
                self.emit_job_event(&job, kind);
            }

            Ok(job)
        }
        .await;

        self.finish_mutation(job_id, &lock, result)
    }

    pub(super) async fn mutate_job_terminal<F>(
        &self,
        job_id: &DomainJobId,
        outcome: domain::job::TerminalOutcome,
        kind: ports::job_scheduler::JobLifecycleEventKind,
        action: F,
    ) -> Result<Job, PortError>
    where
        F: FnOnce(&mut Job) -> Result<(), domain::error::DomainError>,
    {
        let lock = self.mutation_locks.get_lock(job_id)?;
        let result = async {
            let _guard = lock.lock().await;

            let mut job = self
                .repo
                .get(job_id)
                .await?
                .ok_or_else(|| PortError::NotFound {
                    resource: format!("Job {}", job_id),
                })?;

            let expected_revision = job.revision();
            action(&mut job).map_err(|e| PortError::Unexpected {
                message: e.to_string(),
            })?;

            if job.revision() != expected_revision {
                let command = ports::transaction::CommitTerminalJobUpdate {
                    deduplication_key: format!("terminal-{}-{:?}", job.id(), outcome),
                    project_id: job.project_id().clone(),
                    job: job.clone(),
                    expected_revision,
                    outcome,
                };

                self.storage_uow.commit_terminal_job_update(command).await?;
                self.cache.track_persisted(&job).await;
                self.emit_job_event(&job, kind);
            }

            Ok(job)
        }
        .await;

        self.finish_mutation(job_id, &lock, result)
    }

    fn finish_mutation<T>(
        &self,
        job_id: &DomainJobId,
        lock: &Arc<tokio::sync::Mutex<()>>,
        result: Result<T, PortError>,
    ) -> Result<T, PortError> {
        match (result, self.mutation_locks.release_if_unused(job_id, lock)) {
            (Ok(value), Ok(())) => Ok(value),
            (Ok(_), Err(cleanup_error)) => Err(cleanup_error),
            (Err(error), _) => Err(error),
        }
    }

    pub async fn remove_cancel_handle(&self, id: &DomainJobId) {
        self.runtime_registry.lock_entries().entries.remove(id);
    }

    pub(super) async fn cancel_runtime_and_wait(
        &self,
        job_id: &DomainJobId,
    ) -> Option<ports::job_runtime_control::RuntimeTaskOutcome> {
        let completion = {
            let mut registry = self.runtime_registry.lock_entries();
            match registry.entries.get(job_id) {
                Some(super::runtime_registry::JobRuntimeEntry::Attached { task, .. }) => {
                    task.cancel.cancel();
                    Some(task.completion.clone())
                }
                Some(super::runtime_registry::JobRuntimeEntry::Reserved { .. }) => {
                    registry.entries.remove(job_id);
                    None
                }
                None => None,
            }
        };

        match completion {
            Some(completion) => Some(completion.wait().await),
            None => None,
        }
    }

    pub(super) fn emit_job_event(
        &self,
        job: &Job,
        kind: ports::job_scheduler::JobLifecycleEventKind,
    ) {
        let scheduled = map_job_to_scheduled(job);

        let event = JobLifecycleEvent {
            kind,
            job: scheduled,
        };

        if let Some(emitter) = &self.emitter {
            emitter(event);
        }
    }
}
