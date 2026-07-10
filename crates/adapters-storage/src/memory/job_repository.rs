use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use domain::job::{Job, JobId};
use domain::project::ProjectId;
use ports::error::PortError;
use ports::repository::JobRepository;

#[derive(Clone)]
pub struct InMemoryJobRepository {
    pub jobs: Arc<Mutex<HashMap<JobId, Job>>>,
}

impl InMemoryJobRepository {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryJobRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl JobRepository for InMemoryJobRepository {
    async fn create(&self, job: Job) -> Result<Job, PortError> {
        let mut lock = self.jobs.lock().unwrap();
        lock.insert(job.id().clone(), job.clone());
        Ok(job)
    }

    async fn get(&self, id: &JobId) -> Result<Option<Job>, PortError> {
        let lock = self.jobs.lock().unwrap();
        Ok(lock.get(id).cloned())
    }

    async fn save(&self, job: &Job) -> Result<(), PortError> {
        let mut lock = self.jobs.lock().unwrap();
        lock.insert(job.id().clone(), job.clone());
        Ok(())
    }

    async fn list_by_project(&self, project_id: &ProjectId) -> Result<Vec<Job>, PortError> {
        let lock = self.jobs.lock().unwrap();
        Ok(lock
            .values()
            .filter(|j| j.project_id() == project_id)
            .cloned()
            .collect())
    }

    async fn list_active(&self) -> Result<Vec<Job>, PortError> {
        let lock = self.jobs.lock().unwrap();
        Ok(lock
            .values()
            .filter(|j| j.status() == &domain::job::JobStatus::Running)
            .cloned()
            .collect())
    }

    async fn list_recent(&self, limit: usize) -> Result<Vec<Job>, PortError> {
        let lock = self.jobs.lock().unwrap();
        let mut jobs: Vec<Job> = lock.values().cloned().collect();
        jobs.sort_by_key(|b| std::cmp::Reverse(*b.created_at()));
        Ok(jobs.into_iter().take(limit).collect())
    }
}
