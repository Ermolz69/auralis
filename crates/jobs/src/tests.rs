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
    CommitPipelineStartFailure, CommitProjectDelete, CommitStagedArtifactWrite,
    CommitTerminalJobUpdate, CommitTranscriptImport, StorageUnitOfWork,
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
        _c: CommitStagedArtifactWrite,
    ) -> Result<(), PortError> {
        Ok(())
    }
    async fn commit_managed_source_import(
        &self,
        _c: CommitManagedSourceImport,
    ) -> Result<(), PortError> {
        Ok(())
    }
    async fn commit_project_delete(&self, _c: CommitProjectDelete) -> Result<(), PortError> {
        Ok(())
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

    async fn save(&self, job: &Job) -> Result<(), PortError> {
        self.jobs.lock().await.insert(job.id().clone(), job.clone());
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
        async fn save(&self, job: &Job) -> Result<(), PortError> {
            tokio::time::sleep(Duration::from_millis(10)).await;
            self.inner.save(job).await
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
