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
    let manager = JobManager::new(repo, None);

    let job_id = manager
        .start_mock_dubbing_job_internal("Test Job".to_string(), None)
        .await
        .unwrap();

    let job = manager.get_job_internal(&job_id).await.unwrap();
    assert!(*job.status() == JobStatus::Pending || *job.status() == JobStatus::Running);

    let _cancelled_job = manager.cancel_job_internal(&job_id).await.unwrap();

    sleep(Duration::from_millis(600)).await;

    let job = manager.get_job_internal(&job_id).await.unwrap();
    assert_eq!(*job.status(), JobStatus::Cancelled);
}

#[tokio::test]
async fn test_cancel_unknown_job() {
    let repo = Arc::new(MockJobRepository::new());
    let manager = JobManager::new(repo, None);
    let fake_id = JobId::new();

    let result = manager.cancel_job_internal(&fake_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_concurrent_updates_and_cancellation() {
    let repo = Arc::new(MockJobRepository::new());
    let manager = JobManager::new(repo, None);
    let job_id = manager
        .start_mock_dubbing_job_internal("Concurrent Test".to_string(), None)
        .await
        .unwrap();

    let manager_clone = manager.clone();
    let id_clone = job_id.clone();

    let mut tasks = vec![];
    for _ in 0..10 {
        let m = manager.clone();
        let id = job_id.clone();
        tasks.push(tokio::spawn(async move {
            for j in 0..20 {
                if let Some(mut job) = m.get_job_internal(&id).await {
                    let mut prog = job.progress().clone();
                    prog.percent = j as u8;
                    job.update_progress(prog).ok();
                    m.update_job(job).await;
                }
                tokio::task::yield_now().await;
            }
        }));
    }

    tasks.push(tokio::spawn(async move {
        sleep(Duration::from_millis(5)).await;
        let _ = manager_clone.cancel_job_internal(&id_clone).await;
    }));

    for t in tasks {
        let _ = t.await;
    }

    let job = manager.get_job_internal(&job_id).await.unwrap();
    assert!(
        *job.status() == JobStatus::Cancelled
            || *job.status() == JobStatus::Running
            || *job.status() == JobStatus::Pending
            || *job.status() == JobStatus::Completed
    );
}

#[tokio::test]
async fn test_enqueue_existing_job_starts_pending_job() {
    let repo = Arc::new(MockJobRepository::new());
    let manager = JobManager::new(repo.clone(), None);

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
    let manager = JobManager::new(repo, None);

    let result = manager.enqueue_existing_job(&JobId::new()).await;
    assert!(matches!(result, Err(PortError::NotFound { .. })));
}

#[tokio::test]
async fn test_enqueue_existing_job_rejects_completed_job() {
    let repo = Arc::new(MockJobRepository::new());
    let manager = JobManager::new(repo.clone(), None);

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
    let manager = JobManager::new(repo.clone(), None);

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
