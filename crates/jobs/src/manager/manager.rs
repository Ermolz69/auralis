use std::str::FromStr;
use std::sync::Arc;

use domain::job::{Job, JobId as DomainJobId, JobKind};
use domain::project::ProjectId as DomainProjectId;
use ports::error::PortError;
use ports::job_scheduler::JobLifecycleEvent;
use ports::repository::JobRepository;

use super::cache::JobCache;
use super::mapper::map_job_to_scheduled;
use super::runtime_registry::RuntimeRegistry;
pub type JobEventEmitter = Arc<dyn Fn(JobLifecycleEvent) + Send + Sync + 'static>;

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
        self.emit_job_event(&job, ports::job_scheduler::JobLifecycleEventKind::Created);

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
            Err(_e) => {
                tracing::warn!(
                    error = %common::observability::redaction::DiagnosticError {
                        kind: "RepositoryListRecentJobsFailed",
                        code: None,
                        retryable: true,
                    },
                    "Failed to list recent jobs from repository"
                );
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
        kind: ports::job_scheduler::JobLifecycleEventKind,
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

        let expected_revision = job.revision();

        action(&mut job).map_err(|e| PortError::Unexpected {
            message: e.to_string(),
        })?;

        self.repo.save(&job, expected_revision).await?;
        self.cache.insert(job.clone()).await;
        self.emit_job_event(&job, kind);

        Ok(job)
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
        let lock = self.mutation_locks.get_lock(job_id);
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

        let deduplication_key = format!("terminal-{}-{:?}", job.id(), outcome);

        let command = ports::transaction::CommitTerminalJobUpdate {
            job: job.clone(),
            expected_revision,
            deduplication_key,
            project_id: job.project_id().clone(),
            outcome,
        };

        self.storage_uow.commit_terminal_job_update(command).await?;

        self.cache.insert(job.clone()).await;
        self.emit_job_event(&job, kind);

        Ok(job)
    }

    pub async fn remove_cancel_handle(&self, id: &DomainJobId) {
        self.runtime_registry.lock_entries().entries.remove(id);
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

#[async_trait::async_trait]
impl ports::job_runtime_control::JobRuntimeControlPort for JobManager {
    async fn reserve(
        &self,
        job_id: domain::job::JobId,
        project_id: domain::project::ProjectId,
    ) -> Result<(), ports::error::PortError> {
        let mut inner = self.runtime_registry.lock_inner();
        if inner.closed {
            return Err(ports::error::PortError::Unexpected {
                message: "Admission closed/shutting down".to_string(),
            });
        }
        if inner.entries.contains_key(&job_id) {
            return Err(ports::error::PortError::Conflict {
                resource: format!("JobRuntimeEntry {}", job_id),
                message: "Job is already registered in the runtime".to_string(),
            });
        }
        inner.entries.insert(
            job_id,
            super::runtime_registry::JobRuntimeEntry::Reserved { project_id },
        );
        Ok(())
    }

    async fn attach_task(
        &self,
        job_id: domain::job::JobId,
        task: ports::job_runtime_control::RuntimeTask,
    ) -> Result<(), ports::job_runtime_control::AttachTaskError> {
        let mut inner = self.runtime_registry.lock_inner();
        if inner.closed {
            return Err(ports::job_runtime_control::AttachTaskError {
                source: ports::error::PortError::Unexpected {
                    message: "Admission closed/shutting down".to_string(),
                },
                task,
            });
        }
        match inner.entries.remove(&job_id) {
            Some(super::runtime_registry::JobRuntimeEntry::Reserved { project_id }) => {
                if task.cancel.is_cancelled() {
                    return Err(ports::job_runtime_control::AttachTaskError {
                        source: ports::error::PortError::Conflict {
                            resource: format!("JobRuntimeEntry {}", job_id),
                            message: "Task is already cancelled before attach".to_string(),
                        },
                        task,
                    });
                }
                inner.entries.insert(
                    job_id,
                    super::runtime_registry::JobRuntimeEntry::Attached { project_id, task },
                );
                Ok(())
            }
            Some(other) => {
                inner.entries.insert(job_id.clone(), other);
                Err(ports::job_runtime_control::AttachTaskError {
                    source: ports::error::PortError::Conflict {
                        resource: format!("JobRuntimeEntry {}", job_id),
                        message: "Job is already attached".to_string(),
                    },
                    task,
                })
            }
            None => Err(ports::job_runtime_control::AttachTaskError {
                source: ports::error::PortError::NotFound {
                    resource: format!("JobRuntimeEntry {}", job_id),
                },
                task,
            }),
        }
    }

    fn finish_now(&self, job_id: &domain::job::JobId) {
        self.runtime_registry.lock_entries().entries.remove(job_id);
    }

    async fn rollback_runtime_start(
        &self,
        job_id: &domain::job::JobId,
    ) -> Result<ports::job_runtime_control::RuntimeCleanupOutcome, ports::error::PortError> {
        let entry_opt = self.runtime_registry.lock_entries().entries.remove(job_id);

        match entry_opt {
            Some(super::runtime_registry::JobRuntimeEntry::Attached { task, .. }) => {
                task.cancel.cancel();
                task.join_handle.abort();
                let _ = task.join_handle.await;
                Ok(ports::job_runtime_control::RuntimeCleanupOutcome::ForcedAborted)
            }
            Some(super::runtime_registry::JobRuntimeEntry::Reserved { .. }) => {
                Ok(ports::job_runtime_control::RuntimeCleanupOutcome::ReservationRemoved)
            }
            None => Ok(ports::job_runtime_control::RuntimeCleanupOutcome::Missing),
        }
    }

    async fn cancel_and_evict_jobs(
        &self,
        job_ids: &[domain::job::JobId],
    ) -> Result<ports::job_runtime_control::RuntimeCleanupReport, ports::error::PortError> {
        let mut report = ports::job_runtime_control::RuntimeCleanupReport {
            jobs: std::collections::HashMap::new(),
        };

        struct ReaperEntry {
            job_id: domain::job::JobId,
            join_handle: tokio::task::JoinHandle<ports::job_runtime_control::RuntimeTaskOutcome>,
            abort_handle: tokio::task::AbortHandle,
        }

        let mut reaper_entries = Vec::new();

        {
            let mut registry = self.runtime_registry.lock_entries();
            for job_id in job_ids {
                match registry.entries.remove(job_id) {
                    Some(super::runtime_registry::JobRuntimeEntry::Attached { task, .. }) => {
                        task.cancel.cancel();
                        let abort_handle = task.join_handle.abort_handle();
                        reaper_entries.push(ReaperEntry {
                            job_id: job_id.clone(),
                            join_handle: task.join_handle,
                            abort_handle,
                        });
                    }
                    Some(super::runtime_registry::JobRuntimeEntry::Reserved { .. }) => {
                        report.jobs.insert(
                            job_id.clone(),
                            ports::job_runtime_control::RuntimeCleanupOutcome::ReservationRemoved,
                        );
                    }
                    None => {
                        report.jobs.insert(
                            job_id.clone(),
                            ports::job_runtime_control::RuntimeCleanupOutcome::Missing,
                        );
                    }
                }
            }
        }

        // 1. Cooperative Wait Phase
        let cooperative_timeout = std::time::Duration::from_secs(5);
        let _ = tokio::time::timeout(cooperative_timeout, async {
            let mut futs = Vec::new();
            for entry in &mut reaper_entries {
                futs.push(async {
                    let _ = (&mut entry.join_handle).await;
                });
            }
            futures::future::join_all(futs).await;
        })
        .await;

        // Extract finished entries after cooperative timeout
        let mut remaining = Vec::new();
        for mut entry in reaper_entries.into_iter() {
            if entry.join_handle.is_finished() {
                let outcome = match (&mut entry.join_handle).await {
                    Ok(ports::job_runtime_control::RuntimeTaskOutcome::Completed) => {
                        ports::job_runtime_control::RuntimeCleanupOutcome::Completed
                    }
                    Ok(ports::job_runtime_control::RuntimeTaskOutcome::Cancelled) => {
                        ports::job_runtime_control::RuntimeCleanupOutcome::CooperativeCancelled
                    }
                    Ok(ports::job_runtime_control::RuntimeTaskOutcome::DeletedNoOp) => {
                        ports::job_runtime_control::RuntimeCleanupOutcome::Completed
                    }
                    Ok(ports::job_runtime_control::RuntimeTaskOutcome::ApplicationFailed) => {
                        ports::job_runtime_control::RuntimeCleanupOutcome::JoinFailed
                    }
                    Ok(ports::job_runtime_control::RuntimeTaskOutcome::RecoveryRequired) => {
                        ports::job_runtime_control::RuntimeCleanupOutcome::JoinFailed
                    }
                    Ok(ports::job_runtime_control::RuntimeTaskOutcome::Panicked) => {
                        ports::job_runtime_control::RuntimeCleanupOutcome::Panicked
                    }
                    Err(e) if e.is_panic() => {
                        ports::job_runtime_control::RuntimeCleanupOutcome::Panicked
                    }
                    Err(e) if e.is_cancelled() => {
                        ports::job_runtime_control::RuntimeCleanupOutcome::ForcedAborted
                    }
                    Err(_) => ports::job_runtime_control::RuntimeCleanupOutcome::JoinFailed,
                };
                report.jobs.insert(entry.job_id, outcome);
            } else {
                remaining.push(entry);
            }
        }

        // 2. Forced Abort Phase
        for entry in &remaining {
            entry.abort_handle.abort();
        }

        let abort_timeout = std::time::Duration::from_secs(2);
        let _ = tokio::time::timeout(abort_timeout, async {
            let mut futs = Vec::new();
            for entry in &mut remaining {
                futs.push(async {
                    let _ = (&mut entry.join_handle).await;
                });
            }
            futures::future::join_all(futs).await;
        })
        .await;

        // Extract finished entries after abort timeout
        let mut unconfirmed = Vec::new();
        for mut entry in remaining.into_iter() {
            if entry.join_handle.is_finished() {
                let outcome = match (&mut entry.join_handle).await {
                    Ok(ports::job_runtime_control::RuntimeTaskOutcome::Cancelled) => {
                        ports::job_runtime_control::RuntimeCleanupOutcome::ForcedAborted
                    }
                    Ok(ports::job_runtime_control::RuntimeTaskOutcome::Completed) => {
                        ports::job_runtime_control::RuntimeCleanupOutcome::Completed
                    }
                    Ok(ports::job_runtime_control::RuntimeTaskOutcome::DeletedNoOp) => {
                        ports::job_runtime_control::RuntimeCleanupOutcome::Completed
                    }
                    Ok(ports::job_runtime_control::RuntimeTaskOutcome::ApplicationFailed) => {
                        ports::job_runtime_control::RuntimeCleanupOutcome::JoinFailed
                    }
                    Ok(ports::job_runtime_control::RuntimeTaskOutcome::RecoveryRequired) => {
                        ports::job_runtime_control::RuntimeCleanupOutcome::JoinFailed
                    }
                    Ok(ports::job_runtime_control::RuntimeTaskOutcome::Panicked) => {
                        ports::job_runtime_control::RuntimeCleanupOutcome::Panicked
                    }
                    Err(e) if e.is_panic() => {
                        ports::job_runtime_control::RuntimeCleanupOutcome::Panicked
                    }
                    Err(e) if e.is_cancelled() => {
                        ports::job_runtime_control::RuntimeCleanupOutcome::ForcedAborted
                    }
                    Err(_) => ports::job_runtime_control::RuntimeCleanupOutcome::JoinFailed,
                };
                report.jobs.insert(entry.job_id, outcome);
            } else {
                report.jobs.insert(
                    entry.job_id.clone(),
                    ports::job_runtime_control::RuntimeCleanupOutcome::Unconfirmed,
                );
                unconfirmed.push(entry);
            }
        }

        // Send unconfirmed handles to background quarantine (fire and forget for now, but avoids detaching them locally by leaking or moving to a static registry)
        if !unconfirmed.is_empty() {
            // Note: Quarantine registry should take ownership.
            // In a full implementation we'd pass them to a dedicated QuarantineRegistry.
            // For now, we spawn a detached background task that awaits them to ensure they eventually clean up.
            let cache = self.cache.clone();
            let locks = self.mutation_locks.clone();
            tokio::spawn(async move {
                for mut entry in unconfirmed {
                    let _ = (&mut entry.join_handle).await;
                    cache.remove(&entry.job_id).await;
                    locks.remove(&entry.job_id);
                }
            });
        }

        // Evict cache and locks only for fully confirmed stopped jobs
        for job_id in job_ids {
            let outcome = report.jobs.get(job_id);
            if let Some(ports::job_runtime_control::RuntimeCleanupOutcome::Unconfirmed) = outcome {
                continue;
            }

            let lock = self.mutation_locks.get_lock(job_id);
            let _guard = lock.lock().await;

            self.cache.remove(job_id).await;

            drop(_guard);
            self.mutation_locks.remove(job_id);
        }

        Ok(report)
    }

    async fn drain_all(
        &self,
        deadline: std::time::Duration,
    ) -> Result<ports::job_runtime_control::RuntimeShutdownReport, ports::error::PortError> {
        let start_time = std::time::Instant::now();
        let mut reaper_entries = Vec::new();
        let mut report = ports::job_runtime_control::RuntimeShutdownReport::default();

        let entries = match self.runtime_registry.try_close() {
            Some(e) => e,
            None => return Err(ports::error::PortError::AlreadyStopped),
        };

        for (job_id, entry) in entries {
            match entry {
                super::runtime_registry::JobRuntimeEntry::Reserved { .. } => {
                    report.reservation_removed_count += 1;
                    self.cache.remove(&job_id).await;
                    self.mutation_locks.remove(&job_id);
                }
                super::runtime_registry::JobRuntimeEntry::Attached { task, .. } => {
                    task.cancel.cancel();
                    reaper_entries.push((job_id, task));
                }
            }
        }

        let mut abort_handles = std::collections::HashMap::new();
        let mut stream = futures::stream::FuturesUnordered::new();

        for (job_id, task) in reaper_entries {
            abort_handles.insert(job_id.clone(), task.join_handle.abort_handle());
            stream.push(async move {
                let res = task.join_handle.await;
                (job_id, res)
            });
        }

        // 1. Cooperative Wait Phase
        let elapsed = start_time.elapsed();
        let total_budget = deadline
            .checked_sub(elapsed)
            .unwrap_or(std::time::Duration::ZERO);

        let cooperative_timeout = if total_budget > std::time::Duration::from_millis(500) {
            total_budget.mul_f64(0.8)
        } else {
            std::time::Duration::ZERO
        };

        if !abort_handles.is_empty() && cooperative_timeout > std::time::Duration::ZERO {
            let timeout_fut = tokio::time::sleep(cooperative_timeout);
            tokio::pin!(timeout_fut);

            loop {
                tokio::select! {
                    _ = &mut timeout_fut => {
                        break;
                    }
                    res_opt = futures::StreamExt::next(&mut stream) => {
                        match res_opt {
                            Some((job_id, join_res)) => {
                                abort_handles.remove(&job_id);
                                classify_outcome(join_res, &mut report, false);

                                let lock = self.mutation_locks.get_lock(&job_id);
                                let _guard = lock.lock().await;
                                self.cache.remove(&job_id).await;
                                drop(_guard);
                                self.mutation_locks.remove(&job_id);
                            }
                            None => {
                                break;
                            }
                        }
                    }
                }
            }
        }

        // 2. Forced Abort Phase
        if !abort_handles.is_empty() {
            for abort_handle in abort_handles.values() {
                abort_handle.abort();
            }

            let elapsed = start_time.elapsed();
            let abort_budget = deadline
                .checked_sub(elapsed)
                .unwrap_or(std::time::Duration::ZERO);

            if abort_budget > std::time::Duration::ZERO {
                let abort_timeout_fut = tokio::time::sleep(abort_budget);
                tokio::pin!(abort_timeout_fut);

                loop {
                    tokio::select! {
                        _ = &mut abort_timeout_fut => {
                            break;
                        }
                        res_opt = futures::StreamExt::next(&mut stream) => {
                            match res_opt {
                                Some((job_id, join_res)) => {
                                    abort_handles.remove(&job_id);
                                    classify_outcome(join_res, &mut report, true);

                                    let lock = self.mutation_locks.get_lock(&job_id);
                                    let _guard = lock.lock().await;
                                    self.cache.remove(&job_id).await;
                                    drop(_guard);
                                    self.mutation_locks.remove(&job_id);
                                }
                                None => {
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            for job_id in abort_handles.keys() {
                report.unconfirmed_count += 1;

                let lock = self.mutation_locks.get_lock(job_id);
                let _guard = lock.lock().await;
                self.cache.remove(job_id).await;
                drop(_guard);
                self.mutation_locks.remove(job_id);
            }
        }

        Ok(report)
    }
}

fn classify_outcome(
    join_res: Result<ports::job_runtime_control::RuntimeTaskOutcome, tokio::task::JoinError>,
    report: &mut ports::job_runtime_control::RuntimeShutdownReport,
    was_aborted: bool,
) {
    match join_res {
        Ok(ports::job_runtime_control::RuntimeTaskOutcome::Completed) => {
            report.completed_count += 1;
        }
        Ok(ports::job_runtime_control::RuntimeTaskOutcome::Cancelled) => {
            if was_aborted {
                report.forced_aborted_count += 1;
            } else {
                report.cooperative_cancelled_count += 1;
            }
        }
        Ok(ports::job_runtime_control::RuntimeTaskOutcome::DeletedNoOp) => {
            report.completed_count += 1;
        }
        Ok(ports::job_runtime_control::RuntimeTaskOutcome::ApplicationFailed) => {
            report.join_failed_count += 1;
        }
        Ok(ports::job_runtime_control::RuntimeTaskOutcome::RecoveryRequired) => {
            report.join_failed_count += 1;
        }
        Ok(ports::job_runtime_control::RuntimeTaskOutcome::Panicked) => {
            report.panicked_count += 1;
        }
        Err(e) if e.is_panic() => {
            report.panicked_count += 1;
        }
        Err(e) if e.is_cancelled() => {
            report.forced_aborted_count += 1;
        }
        Err(_) => {
            report.join_failed_count += 1;
        }
    }
}
