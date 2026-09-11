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
    async fn evict_stopped_before(
        &self,
        job_id: &DomainJobId,
        deadline: tokio::time::Instant,
    ) -> Result<bool, PortError> {
        let cleanup = async {
            let lock = self.mutation_locks.get_lock(job_id)?;
            let guard = lock.lock().await;
            self.cache.remove(job_id).await;
            drop(guard);
            self.mutation_locks.release_if_unused(job_id, &lock)
        };
        match tokio::time::timeout_at(deadline, cleanup).await {
            Ok(result) => result.map(|_| true),
            Err(_) => Ok(false),
        }
    }
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

        let mut abort_handles = std::collections::HashMap::new();
        let mut reaper = futures::stream::FuturesUnordered::new();

        {
            let mut registry = self.runtime_registry.lock_entries();
            for job_id in job_ids {
                match registry.entries.remove(job_id) {
                    Some(super::runtime_registry::JobRuntimeEntry::Attached { task, .. }) => {
                        task.cancel.cancel();
                        let job_id = job_id.clone();
                        abort_handles.insert(job_id.clone(), task.join_handle.abort_handle());
                        reaper.push(async move {
                            let join_result = task.join_handle.await;
                            (job_id, join_result)
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

        if !abort_handles.is_empty() {
            let cooperative_timeout = tokio::time::sleep(std::time::Duration::from_secs(5));
            tokio::pin!(cooperative_timeout);

            while !abort_handles.is_empty() {
                tokio::select! {
                    _ = &mut cooperative_timeout => break,
                    result = futures::StreamExt::next(&mut reaper) => {
                        let Some((job_id, join_result)) = result else {
                            break;
                        };
                        abort_handles.remove(&job_id);
                        report.jobs.insert(
                            job_id,
                            classify_cleanup_outcome(join_result, false),
                        );
                    }
                }
            }
        }

        for abort_handle in abort_handles.values() {
            abort_handle.abort();
        }

        if !abort_handles.is_empty() {
            let abort_timeout = tokio::time::sleep(std::time::Duration::from_secs(2));
            tokio::pin!(abort_timeout);

            while !abort_handles.is_empty() {
                tokio::select! {
                    _ = &mut abort_timeout => break,
                    result = futures::StreamExt::next(&mut reaper) => {
                        let Some((job_id, join_result)) = result else {
                            break;
                        };
                        abort_handles.remove(&job_id);
                        report.jobs.insert(
                            job_id,
                            classify_cleanup_outcome(join_result, true),
                        );
                    }
                }
            }
        }

        for job_id in abort_handles.keys() {
            report.jobs.insert(
                job_id.clone(),
                ports::job_runtime_control::RuntimeCleanupOutcome::Unconfirmed,
            );
        }

        if !abort_handles.is_empty() {
            let cache = self.cache.clone();
            let locks = self.mutation_locks.clone();
            tokio::spawn(async move {
                while let Some((job_id, _join_result)) = futures::StreamExt::next(&mut reaper).await
                {
                    cache.remove(&job_id).await;
                    if let Err(_err) = locks.remove_if_unused(&job_id) {
                        tracing::error!(
                            error = %common::observability::redaction::DiagnosticError {
                                kind: "JobMutationLockCleanupFailed",
                                code: None,
                                retryable: false,
                            },
                            "failed to cleanup job mutation lock"
                        );
                    }
                }
            });
        }

        // Evict cache and locks only for fully confirmed stopped jobs
        for job_id in job_ids {
            let outcome = report.jobs.get(job_id);
            if let Some(ports::job_runtime_control::RuntimeCleanupOutcome::Unconfirmed) = outcome {
                continue;
            }

            let lock = self.mutation_locks.get_lock(job_id)?;
            let _guard = lock.lock().await;

            self.cache.remove(job_id).await;

            drop(_guard);
            self.mutation_locks.release_if_unused(job_id, &lock)?;
        }

        Ok(report)
    }

    async fn drain_all(
        &self,
        deadline: std::time::Duration,
    ) -> Result<ports::job_runtime_control::RuntimeShutdownReport, ports::error::PortError> {
        let start_time = std::time::Instant::now();
        let expires_at = tokio::time::Instant::now() + deadline;
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
                    self.mutation_locks.remove_if_unused(&job_id)?;
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

                                if !self.evict_stopped_before(&job_id, expires_at).await? { report.cleanup_deferred_count += 1; }
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

            let abort_budget = deadline
                .checked_sub(start_time.elapsed())
                .unwrap_or(std::time::Duration::ZERO);
            let timeout_fut = tokio::time::sleep(abort_budget);
            tokio::pin!(timeout_fut);

            while !abort_handles.is_empty() && abort_budget > std::time::Duration::ZERO {
                tokio::select! {
                    _ = &mut timeout_fut => break,
                    result = futures::StreamExt::next(&mut stream) => {
                        let Some((job_id, join_res)) = result else {
                            break;
                        };
                        abort_handles.remove(&job_id);
                        classify_outcome(join_res, &mut report, true);

                        if !self.evict_stopped_before(&job_id, expires_at).await? { report.cleanup_deferred_count += 1; }
                    }
                }
            }

            report.unconfirmed_count += abort_handles.len();
        }

        Ok(report)
    }
}

fn classify_cleanup_outcome(
    join_result: Result<ports::job_runtime_control::RuntimeTaskOutcome, tokio::task::JoinError>,
    was_aborted: bool,
) -> ports::job_runtime_control::RuntimeCleanupOutcome {
    use ports::job_runtime_control::RuntimeCleanupOutcome;

    match classify_join_result(join_result, was_aborted) {
        ClassifiedJoinResult::Completed => RuntimeCleanupOutcome::Completed,
        ClassifiedJoinResult::CooperativeCancelled => RuntimeCleanupOutcome::CooperativeCancelled,
        ClassifiedJoinResult::ForcedAborted => RuntimeCleanupOutcome::ForcedAborted,
        ClassifiedJoinResult::Panicked => RuntimeCleanupOutcome::Panicked,
        ClassifiedJoinResult::JoinFailed => RuntimeCleanupOutcome::JoinFailed,
    }
}

enum ClassifiedJoinResult {
    Completed,
    CooperativeCancelled,
    ForcedAborted,
    Panicked,
    JoinFailed,
}

fn classify_join_result(
    join_result: Result<ports::job_runtime_control::RuntimeTaskOutcome, tokio::task::JoinError>,
    was_aborted: bool,
) -> ClassifiedJoinResult {
    use ports::job_runtime_control::RuntimeTaskOutcome;

    match join_result {
        Ok(RuntimeTaskOutcome::Completed | RuntimeTaskOutcome::DeletedNoOp) => {
            ClassifiedJoinResult::Completed
        }
        Ok(RuntimeTaskOutcome::Cancelled) if was_aborted => ClassifiedJoinResult::ForcedAborted,
        Ok(RuntimeTaskOutcome::Cancelled) => ClassifiedJoinResult::CooperativeCancelled,
        Ok(RuntimeTaskOutcome::ApplicationFailed | RuntimeTaskOutcome::RecoveryRequired) => {
            ClassifiedJoinResult::JoinFailed
        }
        Ok(RuntimeTaskOutcome::Panicked) => ClassifiedJoinResult::Panicked,
        Err(error) if error.is_panic() => ClassifiedJoinResult::Panicked,
        Err(error) if error.is_cancelled() => ClassifiedJoinResult::ForcedAborted,
        Err(_) => ClassifiedJoinResult::JoinFailed,
    }
}

fn classify_outcome(
    join_res: Result<ports::job_runtime_control::RuntimeTaskOutcome, tokio::task::JoinError>,
    report: &mut ports::job_runtime_control::RuntimeShutdownReport,
    was_aborted: bool,
) {
    match classify_join_result(join_res, was_aborted) {
        ClassifiedJoinResult::Completed => {
            report.completed_count += 1;
        }
        ClassifiedJoinResult::CooperativeCancelled => {
            report.cooperative_cancelled_count += 1;
        }
        ClassifiedJoinResult::ForcedAborted => {
            report.forced_aborted_count += 1;
        }
        ClassifiedJoinResult::Panicked => {
            report.panicked_count += 1;
        }
        ClassifiedJoinResult::JoinFailed => {
            report.join_failed_count += 1;
        }
    }
}
