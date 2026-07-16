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
                tracing::warn!("Failed to list recent jobs from repo: {}", e);
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
    ) -> Result<ports::job_runtime_control::RuntimeCleanupReport, ports::error::PortError> {
        let mut report = ports::job_runtime_control::RuntimeCleanupReport {
            jobs: std::collections::HashMap::new(),
        };

        let mut to_wait = Vec::new();

        // 1. Send cancel signal to all registered jobs
        {
            let registry = self.cancellation_registry.handles.read().await;
            for job_id in job_ids {
                if let Some(reg) = registry.get(job_id) {
                    if *reg.state_rx.borrow() == ports::job_runtime_control::RuntimeState::Finished
                    {
                        report.jobs.insert(
                            job_id.clone(),
                            ports::job_runtime_control::JobCleanupStatus::AlreadyFinished,
                        );
                    } else {
                        reg.cancel_handle.cancel();
                        to_wait.push((
                            job_id.clone(),
                            reg.state_rx.clone(),
                            reg.abort_handle.clone(),
                        ));
                    }
                } else {
                    report.jobs.insert(
                        job_id.clone(),
                        ports::job_runtime_control::JobCleanupStatus::NotRegistered,
                    );
                }
            }
        }

        // 2. Cooperative wait (5s parallel)
        let mut wait_futs = Vec::new();
        for (id, mut rx, abort) in to_wait {
            wait_futs.push(async move {
                let wait_fut = async {
                    while *rx.borrow() != ports::job_runtime_control::RuntimeState::Finished {
                        if rx.changed().await.is_err() {
                            break;
                        }
                    }
                };

                let res = tokio::time::timeout(std::time::Duration::from_secs(5), wait_fut).await;
                (id, rx, abort, res)
            });
        }

        let results = futures::future::join_all(wait_futs).await;

        let mut remaining = Vec::new();
        for (id, rx, abort, res) in results {
            if res.is_err() {
                remaining.push((id, rx, abort));
            } else {
                report.jobs.insert(
                    id,
                    ports::job_runtime_control::JobCleanupStatus::CooperativelyStopped,
                );
            }
        }

        // 3. Forced Abort for remaining
        let mut unconfirmed = Vec::new();

        let mut abort_futs = Vec::new();
        for (id, mut rx, abort) in remaining {
            abort.abort();

            abort_futs.push(async move {
                let wait_fut = async {
                    while *rx.borrow() != ports::job_runtime_control::RuntimeState::Finished {
                        if rx.changed().await.is_err() {
                            break;
                        }
                    }
                };

                let res = tokio::time::timeout(std::time::Duration::from_secs(2), wait_fut).await;
                (id, res)
            });
        }

        let abort_results = futures::future::join_all(abort_futs).await;

        for (id, res) in abort_results {
            if res.is_err() {
                report.jobs.insert(
                    id.clone(),
                    ports::job_runtime_control::JobCleanupStatus::AbortUnconfirmed,
                );
                unconfirmed.push(id);
            } else {
                report
                    .jobs
                    .insert(id, ports::job_runtime_control::JobCleanupStatus::Aborted);
            }
        }

        // 4. Evict cache and locks for all confirmed stopped jobs
        for job_id in job_ids {
            if unconfirmed.contains(job_id) {
                continue;
            }

            let lock = self.mutation_locks.get_lock(job_id);
            let _guard = lock.lock().await;

            self.cache.remove(job_id).await;
            self.cancellation_registry.unregister(job_id).await;

            drop(_guard);
            self.mutation_locks.remove(job_id);
        }

        Ok(report)
    }

    async fn register_runtime_task(
        &self,
        job_id: domain::job::JobId,
        cancel_handle: ports::cancellation::CancelHandle,
        state_rx: tokio::sync::watch::Receiver<ports::job_runtime_control::RuntimeState>,
        abort_handle: tokio::task::AbortHandle,
    ) {
        self.cancellation_registry
            .register(job_id, cancel_handle, state_rx, abort_handle)
            .await;
    }
}
