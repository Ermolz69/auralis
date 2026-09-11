#![allow(clippy::unwrap_used, clippy::expect_used)]
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};

use crate::manager::JobManager;
use domain::job::{Job, JobId, JobStatus};
use ports::error::PortError;
use ports::job_scheduler::JobSchedulerPort;
use ports::repository::JobRepository;

#[tokio::test(start_paused = true)]
async fn shutdown_deadline_includes_a_held_mutation_lock() {
    use ports::job_runtime_control::{
        JobRuntimeControlPort, RuntimeCompletion, RuntimeTask, RuntimeTaskOutcome,
    };
    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo, uow, None);
    let id = JobId::new();
    manager
        .reserve(id.clone(), domain::project::ProjectId::new())
        .await
        .unwrap();
    let (cancel, _) = ports::cancellation::CancelHandle::new();
    let join_handle = tokio::spawn(async { RuntimeTaskOutcome::Completed });
    tokio::task::yield_now().await;
    manager
        .attach_task(
            id.clone(),
            RuntimeTask {
                cancel,
                join_handle,
                completion: Arc::new(RuntimeCompletion::new()),
            },
        )
        .await
        .unwrap();
    let lock = manager.mutation_lock_for_test(&id).unwrap();
    let guard = lock.lock().await;
    let started = tokio::time::Instant::now();
    let report = manager.drain_all(Duration::from_millis(100)).await.unwrap();
    assert!(started.elapsed() <= Duration::from_millis(101));
    assert_eq!(report.completed_count, 1);
    assert_eq!(report.cleanup_deferred_count, 1);
    assert_eq!(report.unconfirmed_count, 0);
    drop(guard);
}
use ports::transaction::{
    ApplyTerminalLifecycle, CommitJobUpdate, CommitManagedSourceImport, CommitPipelineStart,
    CommitPipelineStartFailure, CommitProjectDelete, CommitTerminalJobUpdate,
    CommitTranscriptImport, StorageUnitOfWork,
};

pub struct MockStorageUnitOfWork {
    jobs: Arc<Mutex<HashMap<JobId, Job>>>,
    terminal_commits: Arc<std::sync::atomic::AtomicUsize>,
}

impl MockStorageUnitOfWork {
    pub fn new(jobs: Arc<Mutex<HashMap<JobId, Job>>>) -> Self {
        Self {
            jobs,
            terminal_commits: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    pub fn terminal_commit_count(&self) -> usize {
        self.terminal_commits
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[async_trait]
impl StorageUnitOfWork for MockStorageUnitOfWork {
    async fn commit_youtube_import(
        &self,
        _command: ports::transaction::CommitYoutubeImport,
    ) -> Result<(), PortError> {
        unreachable!()
    }
    async fn commit_transcript_import(&self, _c: CommitTranscriptImport) -> Result<(), PortError> {
        Ok(())
    }
    async fn commit_staged_artifact_write(
        &self,
        _command: ports::transaction::CommitStagedArtifactWrite,
    ) -> Result<(), PortError> {
        Ok(())
    }
    async fn commit_artifact_finalize(
        &self,
        _command: ports::transaction::CommitArtifactFinalize,
    ) -> Result<ports::transaction::CommitArtifactFinalizeResult, PortError> {
        Ok(ports::transaction::CommitArtifactFinalizeResult::Committed)
    }
    async fn commit_managed_source_import(
        &self,
        _c: CommitManagedSourceImport,
    ) -> Result<(), PortError> {
        Ok(())
    }
    async fn commit_project_delete(
        &self,
        _command: CommitProjectDelete,
    ) -> Result<ports::transaction::CommitProjectDeleteResult, PortError> {
        Ok(ports::transaction::CommitProjectDeleteResult {
            deleted_job_ids: vec![],
        })
    }
    async fn commit_job_update(&self, _c: CommitJobUpdate) -> Result<(), PortError> {
        Ok(())
    }
    async fn commit_pipeline_start(&self, _c: CommitPipelineStart) -> Result<(), PortError> {
        Ok(())
    }
    async fn commit_pipeline_start_failure(
        &self,
        _c: CommitPipelineStartFailure,
    ) -> Result<(), PortError> {
        Ok(())
    }
    async fn commit_terminal_job_update(
        &self,
        c: CommitTerminalJobUpdate,
    ) -> Result<(), PortError> {
        self.terminal_commits
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.jobs
            .lock()
            .await
            .insert(c.job.id().clone(), c.job.clone());
        Ok(())
    }
    async fn apply_terminal_lifecycle_conditionally(
        &self,
        _c: ApplyTerminalLifecycle,
    ) -> Result<domain::project::status::TerminalTransitionResult, PortError> {
        Ok(domain::project::status::TerminalTransitionResult::AlreadyApplied)
    }
}

pub struct MockJobRepository {
    jobs: Arc<Mutex<HashMap<JobId, Job>>>,
    fail_get: Arc<std::sync::atomic::AtomicBool>,
    fail_save: Arc<std::sync::atomic::AtomicBool>,
    fail_list_recent: Arc<std::sync::atomic::AtomicBool>,
}

impl MockJobRepository {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(HashMap::new())),
            fail_get: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            fail_save: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            fail_list_recent: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn fail_get(&self) {
        self.fail_get
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn fail_save(&self) {
        self.fail_save
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn fail_list_recent(&self) {
        self.fail_list_recent
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    fn outage() -> PortError {
        PortError::Storage {
            operation: "job_repository",
            message: "database unavailable".to_string(),
        }
    }
}

#[async_trait]
impl JobRepository for MockJobRepository {
    async fn create(&self, job: Job) -> Result<Job, PortError> {
        self.jobs.lock().await.insert(job.id().clone(), job.clone());
        Ok(job)
    }

    async fn get(&self, id: &JobId) -> Result<Option<Job>, PortError> {
        if self.fail_get.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(Self::outage());
        }
        Ok(self.jobs.lock().await.get(id).cloned())
    }

    async fn save(&self, job: &Job, expected_revision: u64) -> Result<(), PortError> {
        if self.fail_save.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(Self::outage());
        }
        let mut db = self.jobs.lock().await;

        let existing = db.get(job.id()).ok_or_else(|| PortError::Unexpected {
            message: format!("Job {} not found during save", job.id()),
        })?;

        if existing.revision() != expected_revision {
            return Err(PortError::Conflict {
                resource: "Job".to_string(),
                message: format!("Optimistic concurrency conflict for job id {}", job.id()),
            });
        }

        db.insert(job.id().clone(), job.clone());
        Ok(())
    }

    async fn list_by_project(
        &self,
        _project_id: &domain::project::ProjectId,
    ) -> Result<Vec<Job>, PortError> {
        Ok(vec![])
    }

    async fn list_active(&self) -> Result<Vec<Job>, PortError> {
        Ok(vec![])
    }

    async fn list_recent(&self, limit: usize) -> Result<Vec<Job>, PortError> {
        if self
            .fail_list_recent
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            return Err(Self::outage());
        }
        let mut jobs: Vec<_> = self.jobs.lock().await.values().cloned().collect();
        jobs.sort_by_key(|job| std::cmp::Reverse(*job.created_at()));
        Ok(jobs.into_iter().take(limit).collect())
    }
}

async fn insert_newer_repository_jobs(
    repo: &MockJobRepository,
    after: chrono::DateTime<chrono::Utc>,
    count: usize,
) {
    for offset in 1..=count {
        let job = Job::new(
            domain::project::ProjectId::new(),
            format!("Recent job {offset}"),
            domain::job::JobKind::Dubbing,
        );
        let mut snapshot = job.to_snapshot();
        snapshot.created_at = after + chrono::Duration::seconds(offset as i64);
        snapshot.updated_at = snapshot.created_at;
        repo.create(Job::from_snapshot(snapshot)).await.unwrap();
    }
}

#[tokio::test]
async fn test_job_manager_flow() {
    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo.clone(), uow, None);

    let job = manager
        .start_dubbing_job(ports::job_scheduler::StartDubbingJobRequest {
            title: "Test Job".into(),
            project_id: Some(domain::project::ProjectId::new()),
        })
        .await
        .unwrap();
    let job_id = job.id.clone();

    let job = manager.get_job(&job_id).await.unwrap().unwrap();
    assert!(job.status == JobStatus::Pending || job.status == JobStatus::Running);

    let _cancelled_job = manager.cancel_job(&job_id).await.unwrap();

    let job = manager.get_job(&job_id).await.unwrap().unwrap();
    assert_eq!(job.status, JobStatus::Cancelled);
}

#[tokio::test]
async fn test_cancel_unknown_job() {
    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo.clone(), uow, None);
    let fake_id = JobId::new();

    let result = manager.cancel_job(&fake_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_deterministic_concurrent_updates() {
    struct DelayedRepo {
        inner: MockJobRepository,
    }

    #[async_trait]
    impl JobRepository for DelayedRepo {
        async fn create(&self, job: Job) -> Result<Job, PortError> {
            self.inner.create(job).await
        }
        async fn get(&self, id: &JobId) -> Result<Option<Job>, PortError> {
            // Introduce a delay to ensure overlap if locks weren't used
            tokio::time::sleep(Duration::from_millis(10)).await;
            self.inner.get(id).await
        }
        async fn save(&self, job: &Job, expected_revision: u64) -> Result<(), PortError> {
            tokio::time::sleep(Duration::from_millis(10)).await;
            self.inner.save(job, expected_revision).await
        }
        async fn list_by_project(
            &self,
            project_id: &domain::project::ProjectId,
        ) -> Result<Vec<Job>, PortError> {
            self.inner.list_by_project(project_id).await
        }
        async fn list_active(&self) -> Result<Vec<Job>, PortError> {
            self.inner.list_active().await
        }
        async fn list_recent(&self, limit: usize) -> Result<Vec<Job>, PortError> {
            self.inner.list_recent(limit).await
        }
    }

    let repo = Arc::new(DelayedRepo {
        inner: MockJobRepository::new(),
    });
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.inner.jobs.clone()));
    let manager = JobManager::new(repo, uow, None);

    let job = manager
        .start_dubbing_job(ports::job_scheduler::StartDubbingJobRequest {
            title: "Deterministic Test".into(),
            project_id: Some(domain::project::ProjectId::new()),
        })
        .await
        .unwrap();
    let job_id = job.id.clone();

    // Spawn an update task
    let m_clone = manager.clone();
    let id_clone = job_id.clone();
    let update_task = tokio::spawn(async move {
        m_clone
            .update_job_stage(
                &id_clone,
                domain::dubbing::DubbingPipelineStage::FetchMetadata,
                domain::job::JobProgress {
                    percent: 50,
                    message: "Updating".into(),
                    current_step: None,
                    processed_items: None,
                    total_items: None,
                },
            )
            .await
    });

    // Wait a tiny bit to ensure update_task enters the locked section
    sleep(Duration::from_millis(5)).await;

    // Spawn a cancel task
    let m_clone2 = manager.clone();
    let id_clone2 = job_id.clone();
    let cancel_task = tokio::spawn(async move { m_clone2.cancel_job(&id_clone2).await });

    let update_result = update_task.await.unwrap();
    let cancel_result = cancel_task.await.unwrap();

    // Since cancel_task was queued behind update_task's lock, it sees the updated job and cancels it.
    assert!(update_result.is_ok());
    assert!(cancel_result.is_ok());

    let final_job = manager.get_job(&job_id).await.unwrap().unwrap();
    assert_eq!(final_job.status, JobStatus::Cancelled);
    // The progress from the update task should be present because they were serialized
    assert_eq!(final_job.progress.percent, 50);
}

#[tokio::test]
async fn get_job_propagates_repository_error_even_when_cached() {
    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo.clone(), uow, None);

    let job = manager
        .start_dubbing_job(ports::job_scheduler::StartDubbingJobRequest {
            title: "Cached".into(),
            project_id: Some(domain::project::ProjectId::new()),
        })
        .await
        .unwrap();

    repo.fail_get();

    let result = manager.get_job(&job.id).await;
    assert!(matches!(result, Err(PortError::Storage { .. })));
}

#[tokio::test]
async fn list_jobs_propagates_repository_error_even_when_cached() {
    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo.clone(), uow, None);

    manager
        .start_dubbing_job(ports::job_scheduler::StartDubbingJobRequest {
            title: "Cached".into(),
            project_id: Some(domain::project::ProjectId::new()),
        })
        .await
        .unwrap();

    repo.fail_list_recent();

    let result = manager.list_jobs().await;
    assert!(matches!(result, Err(PortError::Storage { .. })));
}

#[tokio::test]
async fn get_job_does_not_return_stale_cache_when_repository_is_missing() {
    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo.clone(), uow, None);

    let job = manager
        .start_dubbing_job(ports::job_scheduler::StartDubbingJobRequest {
            title: "Stale".into(),
            project_id: Some(domain::project::ProjectId::new()),
        })
        .await
        .unwrap();
    repo.jobs.lock().await.remove(&job.id);

    assert!(manager.get_job(&job.id).await.unwrap().is_none());
    assert!(!manager.is_job_cached_for_test(&job.id).await);
}

#[tokio::test]
async fn list_jobs_revalidates_an_active_job_outside_the_recent_window() {
    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo.clone(), uow, None);

    let active_job = manager
        .start_dubbing_job(ports::job_scheduler::StartDubbingJobRequest {
            title: "Old active job".into(),
            project_id: Some(domain::project::ProjectId::new()),
        })
        .await
        .unwrap();
    insert_newer_repository_jobs(&repo, active_job.created_at, 100).await;

    let jobs = manager.list_jobs().await.unwrap();
    assert_eq!(jobs.len(), 101);
    assert!(jobs.iter().any(|candidate| candidate.id == active_job.id));
}

#[tokio::test]
async fn list_jobs_does_not_restore_a_deleted_job_from_cache() {
    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo.clone(), uow, None);

    let job = manager
        .start_dubbing_job(ports::job_scheduler::StartDubbingJobRequest {
            title: "Deleted job".into(),
            project_id: Some(domain::project::ProjectId::new()),
        })
        .await
        .unwrap();
    repo.jobs.lock().await.remove(&job.id);

    let jobs = manager.list_jobs().await.unwrap();

    assert!(jobs.iter().all(|candidate| candidate.id != job.id));
    assert!(!manager.is_job_cached_for_test(&job.id).await);
}

#[tokio::test]
async fn list_jobs_keeps_the_repository_terminal_snapshot_over_an_active_cache_hint() {
    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo.clone(), uow, None);

    let running = manager
        .start_dubbing_job(ports::job_scheduler::StartDubbingJobRequest {
            title: "Externally completed job".into(),
            project_id: Some(domain::project::ProjectId::new()),
        })
        .await
        .unwrap();
    let mut completed = repo.get(&running.id).await.unwrap().unwrap();
    completed.mark_completed().unwrap();
    repo.jobs
        .lock()
        .await
        .insert(completed.id().clone(), completed.clone());

    let jobs = manager.list_jobs().await.unwrap();
    let listed = jobs
        .iter()
        .find(|candidate| candidate.id == running.id)
        .unwrap();

    assert_eq!(listed.status, JobStatus::Completed);
    assert_eq!(listed.revision, completed.revision());
    assert!(!manager.is_job_cached_for_test(&running.id).await);
}

#[tokio::test]
async fn delayed_get_cannot_reintroduce_an_older_active_cache_hint() {
    struct PausedGetRepository {
        inner: Arc<MockJobRepository>,
        pause_next_get: std::sync::atomic::AtomicBool,
        snapshot_captured: tokio::sync::Notify,
        release_snapshot: tokio::sync::Notify,
    }

    impl PausedGetRepository {
        fn new(inner: Arc<MockJobRepository>) -> Self {
            Self {
                inner,
                pause_next_get: std::sync::atomic::AtomicBool::new(true),
                snapshot_captured: tokio::sync::Notify::new(),
                release_snapshot: tokio::sync::Notify::new(),
            }
        }
    }

    #[async_trait]
    impl JobRepository for PausedGetRepository {
        async fn create(&self, job: Job) -> Result<Job, PortError> {
            self.inner.create(job).await
        }

        async fn get(&self, id: &JobId) -> Result<Option<Job>, PortError> {
            let snapshot = self.inner.get(id).await?;
            if self
                .pause_next_get
                .swap(false, std::sync::atomic::Ordering::SeqCst)
            {
                self.snapshot_captured.notify_one();
                self.release_snapshot.notified().await;
            }
            Ok(snapshot)
        }

        async fn save(&self, job: &Job, expected_revision: u64) -> Result<(), PortError> {
            self.inner.save(job, expected_revision).await
        }

        async fn list_by_project(
            &self,
            project_id: &domain::project::ProjectId,
        ) -> Result<Vec<Job>, PortError> {
            self.inner.list_by_project(project_id).await
        }

        async fn list_active(&self) -> Result<Vec<Job>, PortError> {
            self.inner.list_active().await
        }

        async fn list_recent(&self, limit: usize) -> Result<Vec<Job>, PortError> {
            self.inner.list_recent(limit).await
        }
    }

    let inner = Arc::new(MockJobRepository::new());
    let repo = Arc::new(PausedGetRepository::new(inner.clone()));
    let uow = Arc::new(MockStorageUnitOfWork::new(inner.jobs.clone()));
    let manager = JobManager::new(repo.clone(), uow, None);
    let running = manager
        .start_dubbing_job(ports::job_scheduler::StartDubbingJobRequest {
            title: "Delayed read".into(),
            project_id: Some(domain::project::ProjectId::new()),
        })
        .await
        .unwrap();

    let read_manager = manager.clone();
    let read_job_id = running.id.clone();
    let delayed_read = tokio::spawn(async move { read_manager.get_job(&read_job_id).await });
    repo.snapshot_captured.notified().await;

    let completed = manager.complete_job(&running.id).await.unwrap();
    assert_eq!(completed.status, JobStatus::Completed);
    assert!(!manager.is_job_cached_for_test(&running.id).await);

    repo.release_snapshot.notify_one();
    let delayed = delayed_read.await.unwrap().unwrap().unwrap();

    assert_eq!(delayed.status, JobStatus::Running);
    assert!(delayed.revision < completed.revision);
    assert!(!manager.is_job_cached_for_test(&running.id).await);

    let listed = manager
        .list_jobs()
        .await
        .unwrap()
        .into_iter()
        .find(|candidate| candidate.id == running.id)
        .unwrap();
    assert_eq!(listed.status, JobStatus::Completed);
    assert_eq!(listed.revision, completed.revision);
}

#[tokio::test]
async fn list_jobs_propagates_cached_candidate_revalidation_errors() {
    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo.clone(), uow, None);

    let active_job = manager
        .start_dubbing_job(ports::job_scheduler::StartDubbingJobRequest {
            title: "Old active job".into(),
            project_id: Some(domain::project::ProjectId::new()),
        })
        .await
        .unwrap();
    insert_newer_repository_jobs(&repo, active_job.created_at, 100).await;
    repo.fail_get();

    let result = manager.list_jobs().await;

    assert!(matches!(result, Err(PortError::Storage { .. })));
}

#[tokio::test]
async fn failed_save_does_not_update_cache_or_emit_event() {
    use ports::job_scheduler::JobLifecycleEvent;
    use std::sync::Mutex as StdMutex;

    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let events = Arc::new(StdMutex::new(Vec::<JobLifecycleEvent>::new()));
    let event_sink = events.clone();
    let manager = JobManager::new(
        repo.clone(),
        uow,
        Some(Arc::new(move |event| {
            event_sink.lock().unwrap().push(event);
        })),
    );

    let job = manager
        .start_dubbing_job(ports::job_scheduler::StartDubbingJobRequest {
            title: "Save failure".into(),
            project_id: Some(domain::project::ProjectId::new()),
        })
        .await
        .unwrap();
    events.lock().unwrap().clear();

    repo.fail_save();

    let result = manager
        .update_job_stage(
            &job.id,
            domain::dubbing::DubbingPipelineStage::FetchMetadata,
            domain::job::JobProgress {
                percent: 40,
                message: "Updating".into(),
                current_step: None,
                processed_items: None,
                total_items: None,
            },
        )
        .await;

    assert!(matches!(result, Err(PortError::Storage { .. })));
    assert!(events.lock().unwrap().is_empty());
    let persisted = repo.get(&job.id).await.unwrap().unwrap();
    assert_eq!(persisted.progress().percent, job.progress.percent);
    assert!(manager.is_job_cached_for_test(&job.id).await);
}

#[tokio::test]
async fn test_enqueue_existing_job_starts_pending_job() {
    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo.clone(), uow, None);

    let job = Job::new(
        domain::project::ProjectId::new(),
        "Pending".to_string(),
        domain::job::JobKind::Dubbing,
    );
    let job_id = job.id().clone();
    repo.create(job).await.unwrap();

    let scheduled = manager.enqueue_existing_job(&job_id).await.unwrap();
    assert_eq!(scheduled.status, JobStatus::Running);

    let from_repo = repo.get(&job_id).await.unwrap().unwrap();
    assert_eq!(*from_repo.status(), JobStatus::Running);
}

#[tokio::test]
async fn test_enqueue_existing_job_rejects_missing_job() {
    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo.clone(), uow, None);

    let result = manager.enqueue_existing_job(&JobId::new()).await;
    assert!(matches!(result, Err(PortError::NotFound { .. })));
}

#[tokio::test]
async fn test_enqueue_existing_job_rejects_completed_job() {
    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo.clone(), uow, None);

    let mut job = Job::new(
        domain::project::ProjectId::new(),
        "Completed".to_string(),
        domain::job::JobKind::Dubbing,
    );
    job.start().unwrap();
    job.mark_completed().unwrap();
    let job_id = job.id().clone();
    repo.create(job).await.unwrap();

    let result = manager.enqueue_existing_job(&job_id).await;
    assert!(matches!(result, Err(PortError::Unexpected { .. })));
}

#[tokio::test]
async fn test_enqueue_existing_job_is_idempotent_for_already_running_job() {
    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo.clone(), uow, None);

    let mut job = Job::new(
        domain::project::ProjectId::new(),
        "Running".to_string(),
        domain::job::JobKind::Dubbing,
    );
    job.start().unwrap();
    let job_id = job.id().clone();
    repo.create(job).await.unwrap();

    let scheduled = manager.enqueue_existing_job(&job_id).await.unwrap();
    assert_eq!(scheduled.status, JobStatus::Running);
}

#[tokio::test]
async fn test_drain_all_scenarios() {
    use ports::job_runtime_control::{
        JobRuntimeControlPort, RuntimeCompletion, RuntimeTask, RuntimeTaskOutcome,
    };

    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo.clone(), uow, None);

    // 1. Setup reserved, attached, and already completed/cancelled tasks
    let job_reserved = JobId::new();
    let proj_reserved = domain::project::ProjectId::new();
    manager
        .reserve(job_reserved.clone(), proj_reserved)
        .await
        .unwrap();

    // 2. Setup a cooperative task that finishes quickly on cancellation
    let job_coop = JobId::new();
    let proj_coop = domain::project::ProjectId::new();
    manager.reserve(job_coop.clone(), proj_coop).await.unwrap();
    let (cancel_handle, cancel_token) = ports::cancellation::CancelHandle::new();
    let completion = Arc::new(RuntimeCompletion::new());
    let join_handle = tokio::spawn(async move {
        cancel_token.cancelled().await;
        RuntimeTaskOutcome::Cancelled
    });
    manager
        .attach_task(
            job_coop.clone(),
            RuntimeTask {
                cancel: cancel_handle,
                join_handle,
                completion,
            },
        )
        .await
        .unwrap();

    // 3. Setup a panic task
    let job_panic = JobId::new();
    let proj_panic = domain::project::ProjectId::new();
    manager
        .reserve(job_panic.clone(), proj_panic)
        .await
        .unwrap();
    let (cancel_handle_panic, _cancel_token_panic) = ports::cancellation::CancelHandle::new();
    let completion_panic = Arc::new(RuntimeCompletion::new());
    let join_handle_panic = tokio::spawn(async move {
        panic!("test panic");
    });
    manager
        .attach_task(
            job_panic.clone(),
            RuntimeTask {
                cancel: cancel_handle_panic,
                join_handle: join_handle_panic,
                completion: completion_panic,
            },
        )
        .await
        .unwrap();

    // 4. Setup an unresponsive task that hangs
    let job_hang = JobId::new();
    let proj_hang = domain::project::ProjectId::new();
    manager.reserve(job_hang.clone(), proj_hang).await.unwrap();
    let (cancel_handle_hang, _cancel_token_hang) = ports::cancellation::CancelHandle::new();
    let completion_hang = Arc::new(RuntimeCompletion::new());
    let join_handle_hang = tokio::spawn(async move {
        sleep(Duration::from_secs(10)).await;
        RuntimeTaskOutcome::Completed
    });
    manager
        .attach_task(
            job_hang.clone(),
            RuntimeTask {
                cancel: cancel_handle_hang,
                join_handle: join_handle_hang,
                completion: completion_hang,
            },
        )
        .await
        .unwrap();

    // Perform drain_all with a short timeout to trigger abort for hang
    let report = manager.drain_all(Duration::from_secs(2)).await.unwrap();

    assert_eq!(report.reservation_removed_count, 1);
    assert_eq!(report.cooperative_cancelled_count, 1);
    assert_eq!(report.panicked_count, 1);
    assert_eq!(report.forced_aborted_count, 1);
    assert_eq!(report.unconfirmed_count, 0);
    assert_eq!(report.completed_count, 0);

    // Verify zero handles after shutdown
    assert_eq!(manager.runtime_registry.lock_entries().entries.len(), 0);

    // Verify reserve during drain is rejected
    let res = manager
        .reserve(JobId::new(), domain::project::ProjectId::new())
        .await;
    assert!(res.is_err());

    // Verify double drain returns AlreadyStopped
    let res_double = manager.drain_all(Duration::from_millis(100)).await;
    assert!(matches!(res_double, Err(PortError::AlreadyStopped)));
}

#[tokio::test]
async fn cancel_and_evict_consumes_an_already_completed_task_once() {
    use ports::job_runtime_control::{
        JobRuntimeControlPort, RuntimeCleanupOutcome, RuntimeCompletion, RuntimeTask,
        RuntimeTaskOutcome,
    };

    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo, uow, None);
    let job_id = JobId::new();

    manager
        .reserve(job_id.clone(), domain::project::ProjectId::new())
        .await
        .unwrap();
    let (cancel_handle, _cancel_token) = ports::cancellation::CancelHandle::new();
    let join_handle = tokio::spawn(async { RuntimeTaskOutcome::Completed });
    tokio::task::yield_now().await;
    assert!(join_handle.is_finished());

    manager
        .attach_task(
            job_id.clone(),
            RuntimeTask {
                cancel: cancel_handle,
                join_handle,
                completion: Arc::new(RuntimeCompletion::new()),
            },
        )
        .await
        .unwrap();

    let report = manager
        .cancel_and_evict_jobs(std::slice::from_ref(&job_id))
        .await
        .unwrap();

    assert_eq!(
        report.jobs.get(&job_id),
        Some(&RuntimeCleanupOutcome::Completed)
    );
    assert!(manager.runtime_registry.lock_entries().entries.is_empty());
}

#[tokio::test(start_paused = true)]
async fn cancel_and_evict_collects_mixed_outcomes_and_is_repeatable() {
    use ports::job_runtime_control::{
        JobRuntimeControlPort, RuntimeCleanupOutcome, RuntimeCompletion, RuntimeTask,
        RuntimeTaskOutcome,
    };

    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo, uow, None);
    let project_id = domain::project::ProjectId::new();
    let completed_job_id = JobId::new();
    let cooperative_job_id = JobId::new();
    let forced_job_id = JobId::new();
    let reserved_job_id = JobId::new();
    let missing_job_id = JobId::new();

    for job_id in [
        &completed_job_id,
        &cooperative_job_id,
        &forced_job_id,
        &reserved_job_id,
    ] {
        manager
            .reserve(job_id.clone(), project_id.clone())
            .await
            .unwrap();
    }

    let (completed_cancel, _completed_token) = ports::cancellation::CancelHandle::new();
    let completed_handle = tokio::spawn(async { RuntimeTaskOutcome::Completed });
    tokio::task::yield_now().await;
    assert!(completed_handle.is_finished());
    manager
        .attach_task(
            completed_job_id.clone(),
            RuntimeTask {
                cancel: completed_cancel,
                join_handle: completed_handle,
                completion: Arc::new(RuntimeCompletion::new()),
            },
        )
        .await
        .unwrap();

    let (cooperative_cancel, cooperative_token) = ports::cancellation::CancelHandle::new();
    let cooperative_handle = tokio::spawn(async move {
        cooperative_token.cancelled().await;
        RuntimeTaskOutcome::Cancelled
    });
    manager
        .attach_task(
            cooperative_job_id.clone(),
            RuntimeTask {
                cancel: cooperative_cancel,
                join_handle: cooperative_handle,
                completion: Arc::new(RuntimeCompletion::new()),
            },
        )
        .await
        .unwrap();

    let (forced_cancel, _forced_token) = ports::cancellation::CancelHandle::new();
    let forced_handle = tokio::spawn(async {
        std::future::pending::<()>().await;
        RuntimeTaskOutcome::Completed
    });
    manager
        .attach_task(
            forced_job_id.clone(),
            RuntimeTask {
                cancel: forced_cancel,
                join_handle: forced_handle,
                completion: Arc::new(RuntimeCompletion::new()),
            },
        )
        .await
        .unwrap();

    let job_ids = vec![
        forced_job_id.clone(),
        completed_job_id.clone(),
        cooperative_job_id.clone(),
        reserved_job_id.clone(),
        missing_job_id.clone(),
    ];
    let report = manager.cancel_and_evict_jobs(&job_ids).await.unwrap();

    assert_eq!(
        report.jobs.get(&completed_job_id),
        Some(&RuntimeCleanupOutcome::Completed)
    );
    assert_eq!(
        report.jobs.get(&cooperative_job_id),
        Some(&RuntimeCleanupOutcome::CooperativeCancelled)
    );
    assert_eq!(
        report.jobs.get(&forced_job_id),
        Some(&RuntimeCleanupOutcome::ForcedAborted)
    );
    assert_eq!(
        report.jobs.get(&reserved_job_id),
        Some(&RuntimeCleanupOutcome::ReservationRemoved)
    );
    assert_eq!(
        report.jobs.get(&missing_job_id),
        Some(&RuntimeCleanupOutcome::Missing)
    );
    assert!(manager.runtime_registry.lock_entries().entries.is_empty());

    let repeated = manager.cancel_and_evict_jobs(&job_ids).await.unwrap();
    assert!(
        repeated
            .jobs
            .values()
            .all(|outcome| *outcome == RuntimeCleanupOutcome::Missing)
    );
}

#[tokio::test(start_paused = true)]
async fn cancel_and_evict_defers_cache_cleanup_until_an_unconfirmed_task_exits() {
    use ports::job_runtime_control::{
        JobRuntimeControlPort, RuntimeCleanupOutcome, RuntimeCompletion, RuntimeTask,
        RuntimeTaskOutcome,
    };

    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo, uow, None);
    let project_id = domain::project::ProjectId::new();
    let job = manager
        .start_mock_dubbing_job_internal("Unconfirmed cleanup".into(), Some(project_id.to_string()))
        .await
        .unwrap();
    manager
        .reserve(job.id().clone(), job.project_id().clone())
        .await
        .unwrap();

    let release = Arc::new(std::sync::Barrier::new(2));
    let task_release = release.clone();
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let join_handle = tokio::task::spawn_blocking(move || {
        let _ = started_tx.send(());
        task_release.wait();
        RuntimeTaskOutcome::Completed
    });
    started_rx.await.unwrap();
    let (cancel, _token) = ports::cancellation::CancelHandle::new();
    manager
        .attach_task(
            job.id().clone(),
            RuntimeTask {
                cancel,
                join_handle,
                completion: Arc::new(RuntimeCompletion::new()),
            },
        )
        .await
        .unwrap();

    let cleanup_manager = manager.clone();
    let cleanup_job_id = job.id().clone();
    let cleanup = tokio::spawn(async move {
        cleanup_manager
            .cancel_and_evict_jobs(std::slice::from_ref(&cleanup_job_id))
            .await
    });
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(5)).await;
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(2)).await;
    let report = cleanup.await.unwrap().unwrap();

    assert_eq!(
        report.jobs.get(job.id()),
        Some(&RuntimeCleanupOutcome::Unconfirmed)
    );
    assert_eq!(manager.active_cache_len_for_test().await, 1);

    release.wait();
    tokio::time::timeout(Duration::from_secs(1), async {
        while manager.active_cache_len_for_test().await != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn list_recent_repository_error_preserves_storage_category() {
    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo.clone(), uow, None);

    repo.fail_list_recent();

    let result = manager.list_jobs_internal().await;
    assert!(matches!(result, Err(PortError::Storage { .. })));
}

#[tokio::test]
async fn repeated_cancellation_is_idempotent_without_duplicate_writes_or_events() {
    use ports::job_scheduler::{JobLifecycleEvent, JobLifecycleEventKind};
    use std::sync::Mutex as StdMutex;

    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let events = Arc::new(StdMutex::new(Vec::<JobLifecycleEvent>::new()));
    let event_sink = events.clone();
    let manager = JobManager::new(
        repo,
        uow.clone(),
        Some(Arc::new(move |event| {
            event_sink.lock().unwrap().push(event);
        })),
    );
    let started = manager
        .start_dubbing_job(ports::job_scheduler::StartDubbingJobRequest {
            title: "Idempotent cancellation".into(),
            project_id: Some(domain::project::ProjectId::new()),
        })
        .await
        .unwrap();
    events.lock().unwrap().clear();

    let first = manager.cancel_job(&started.id).await.unwrap();
    let second = manager.cancel_job(&started.id).await.unwrap();

    assert_eq!(first.revision, second.revision);
    assert_eq!(second.status, JobStatus::Cancelled);
    assert_eq!(uow.terminal_commit_count(), 1);
    {
        let events = events.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind, JobLifecycleEventKind::Cancelling);
        assert_eq!(events[0].job.status, JobStatus::Cancelling);
        assert_eq!(events[1].kind, JobLifecycleEventKind::Cancelled);
        assert_eq!(events[1].job.status, JobStatus::Cancelled);
    }
    assert_eq!(manager.active_cache_len_for_test().await, 0);
    assert_eq!(manager.mutation_lock_count_for_test().unwrap(), 0);
}

#[tokio::test]
async fn terminal_history_does_not_accumulate_in_runtime_cache_or_lock_map() {
    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo, uow.clone(), None);

    for index in 0..256 {
        let job = manager
            .start_dubbing_job(ports::job_scheduler::StartDubbingJobRequest {
                title: format!("Bounded runtime state {index}"),
                project_id: Some(domain::project::ProjectId::new()),
            })
            .await
            .unwrap();
        manager.complete_job(&job.id).await.unwrap();
    }

    assert_eq!(uow.terminal_commit_count(), 256);
    assert_eq!(manager.active_cache_len_for_test().await, 0);
    assert_eq!(manager.mutation_lock_count_for_test().unwrap(), 0);
    assert!(manager.runtime_registry.lock_entries().entries.is_empty());
}

#[tokio::test]
async fn failed_mutations_release_per_job_locks() {
    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo, uow, None);

    for _ in 0..256 {
        assert!(matches!(
            manager.cancel_job(&JobId::new()).await,
            Err(PortError::NotFound { .. })
        ));
    }

    assert_eq!(manager.mutation_lock_count_for_test().unwrap(), 0);
}

#[tokio::test]
async fn cancellation_waits_for_runtime_exit_before_persisting_cancelled() {
    use ports::job_runtime_control::{
        JobRuntimeControlPort, RuntimeCompletion, RuntimeTask, RuntimeTaskOutcome,
    };

    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo.clone(), uow, None);
    let job = manager
        .start_dubbing_job(ports::job_scheduler::StartDubbingJobRequest {
            title: "Cooperative cancellation".into(),
            project_id: Some(domain::project::ProjectId::new()),
        })
        .await
        .unwrap();
    manager
        .reserve(job.id.clone(), job.project_id.clone().unwrap())
        .await
        .unwrap();
    let (cancel, token) = ports::cancellation::CancelHandle::new();
    let task_manager = manager.clone();
    let task_job_id = job.id.clone();
    let completion = Arc::new(RuntimeCompletion::new());
    let task_completion = completion.clone();
    let (cancellation_seen_tx, cancellation_seen_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let join_handle = tokio::spawn(async move {
        token.cancelled().await;
        let _ = cancellation_seen_tx.send(());
        let _ = release_rx.await;
        task_manager.finish_now(&task_job_id);
        let outcome = task_completion.record_outcome(RuntimeTaskOutcome::Cancelled);
        task_completion.finish(RuntimeTaskOutcome::RecoveryRequired);
        outcome
    });
    manager
        .attach_task(
            job.id.clone(),
            RuntimeTask {
                cancel,
                join_handle,
                completion,
            },
        )
        .await
        .unwrap();

    let cancel_manager = manager.clone();
    let cancel_job_id = job.id.clone();
    let cancellation = tokio::spawn(async move { cancel_manager.cancel_job(&cancel_job_id).await });
    tokio::time::timeout(Duration::from_secs(1), cancellation_seen_rx)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        repo.get(&job.id).await.unwrap().unwrap().status(),
        &JobStatus::Cancelling
    );
    assert!(!cancellation.is_finished());

    release_tx.send(()).unwrap();
    let cancelled = cancellation.await.unwrap().unwrap();

    assert_eq!(cancelled.status, JobStatus::Cancelled);
    assert_eq!(
        repo.get(&job.id).await.unwrap().unwrap().status(),
        &JobStatus::Cancelled
    );
    assert!(manager.runtime_registry.lock_entries().entries.is_empty());
    assert_eq!(manager.active_cache_len_for_test().await, 0);
    assert_eq!(manager.mutation_lock_count_for_test().unwrap(), 0);
}

#[tokio::test]
async fn concurrent_cancel_and_complete_commit_exactly_one_terminal_state() {
    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo.clone(), uow.clone(), None);
    let job = manager
        .start_dubbing_job(ports::job_scheduler::StartDubbingJobRequest {
            title: "Terminal race".into(),
            project_id: Some(domain::project::ProjectId::new()),
        })
        .await
        .unwrap();
    let barrier = Arc::new(tokio::sync::Barrier::new(3));

    let cancel_manager = manager.clone();
    let cancel_id = job.id.clone();
    let cancel_barrier = barrier.clone();
    let cancel_task = tokio::spawn(async move {
        cancel_barrier.wait().await;
        cancel_manager.cancel_job(&cancel_id).await
    });
    let complete_manager = manager.clone();
    let complete_id = job.id.clone();
    let complete_barrier = barrier.clone();
    let complete_task = tokio::spawn(async move {
        complete_barrier.wait().await;
        complete_manager.complete_job(&complete_id).await
    });
    barrier.wait().await;

    let cancel_result = cancel_task.await.unwrap();
    let complete_result = complete_task.await.unwrap();
    assert_ne!(cancel_result.is_ok(), complete_result.is_ok());
    let persisted = repo.get(&job.id).await.unwrap().unwrap();
    assert!(matches!(
        persisted.status(),
        JobStatus::Cancelled | JobStatus::Completed
    ));
    assert_eq!(uow.terminal_commit_count(), 1);
    assert_eq!(manager.active_cache_len_for_test().await, 0);
    assert_eq!(manager.mutation_lock_count_for_test().unwrap(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_returns_at_deadline_when_a_task_is_temporarily_unresponsive() {
    use ports::job_runtime_control::{
        JobRuntimeControlPort, RuntimeCompletion, RuntimeTask, RuntimeTaskOutcome,
    };

    let repo = Arc::new(MockJobRepository::new());
    let uow = Arc::new(MockStorageUnitOfWork::new(repo.jobs.clone()));
    let manager = JobManager::new(repo, uow, None);
    let job_id = JobId::new();
    manager
        .reserve(job_id.clone(), domain::project::ProjectId::new())
        .await
        .unwrap();
    let (cancel, _token) = ports::cancellation::CancelHandle::new();
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let join_handle = tokio::spawn(async move {
        let _ = started_tx.send(());
        std::thread::sleep(Duration::from_millis(300));
        RuntimeTaskOutcome::Completed
    });
    started_rx.await.unwrap();
    manager
        .attach_task(
            job_id,
            RuntimeTask {
                cancel,
                join_handle,
                completion: Arc::new(RuntimeCompletion::new()),
            },
        )
        .await
        .unwrap();

    let started_at = std::time::Instant::now();
    let report = manager.drain_all(Duration::from_millis(50)).await.unwrap();

    assert!(started_at.elapsed() < Duration::from_millis(200));
    assert_eq!(report.unconfirmed_count, 1);
    assert!(manager.runtime_registry.lock_entries().entries.is_empty());
}
