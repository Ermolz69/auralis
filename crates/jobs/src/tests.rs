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
use ports::transaction::{
    ApplyTerminalLifecycle, CommitJobUpdate, CommitManagedSourceImport, CommitPipelineStart,
    CommitPipelineStartFailure, CommitProjectDelete, CommitTerminalJobUpdate,
    CommitTranscriptImport, StorageUnitOfWork,
};

pub struct MockStorageUnitOfWork {
    jobs: Arc<Mutex<HashMap<JobId, Job>>>,
}

impl MockStorageUnitOfWork {
    pub fn new(jobs: Arc<Mutex<HashMap<JobId, Job>>>) -> Self {
        Self { jobs }
    }
}

#[async_trait]
impl StorageUnitOfWork for MockStorageUnitOfWork {
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
}

impl MockJobRepository {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(HashMap::new())),
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
        Ok(self.jobs.lock().await.get(id).cloned())
    }

    async fn save(&self, job: &Job, expected_revision: u64) -> Result<(), PortError> {
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

    async fn list_recent(&self, _limit: usize) -> Result<Vec<Job>, PortError> {
        Ok(vec![])
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
