use async_trait::async_trait;

use std::sync::{Arc, Mutex};

use domain::job::{Job, JobId};
use domain::project::ProjectId;
use ports::error::PortError;
use ports::repository::JobRepository;

use super::database::InMemoryDatabase;

#[derive(Clone)]
pub struct InMemoryJobRepository {
    pub db: Arc<Mutex<InMemoryDatabase>>,
}

impl InMemoryJobRepository {
    pub fn new(db: Arc<Mutex<InMemoryDatabase>>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl JobRepository for InMemoryJobRepository {
    async fn create(&self, job: Job) -> Result<Job, PortError> {
        let mut lock = self.db.lock().unwrap();
        if lock.jobs.contains_key(job.id()) {
            return Err(PortError::Conflict {
                resource: "Job".to_string(),
                message: format!("Job with id {} already exists", job.id()),
            });
        }
        lock.jobs.insert(job.id().clone(), job.clone());
        Ok(job)
    }

    async fn get(&self, id: &JobId) -> Result<Option<Job>, PortError> {
        let lock = self.db.lock().unwrap();
        Ok(lock.jobs.get(id).cloned())
    }

    async fn save(&self, job: &Job, expected_revision: u64) -> Result<(), PortError> {
        let mut db = self.db.lock().unwrap();

        let existing = db.jobs.get(job.id()).ok_or_else(|| PortError::Unexpected {
            message: format!("Job {} not found during save", job.id()),
        })?;

        if existing.revision() != expected_revision {
            return Err(PortError::Conflict {
                resource: "Job".to_string(),
                message: format!("Optimistic concurrency conflict for job id {}", job.id()),
            });
        }

        db.jobs.insert(job.id().clone(), job.clone());
        Ok(())
    }

    async fn list_by_project(&self, project_id: &ProjectId) -> Result<Vec<Job>, PortError> {
        let lock = self.db.lock().unwrap();
        Ok(lock
            .jobs
            .values()
            .filter(|j| j.project_id() == project_id)
            .cloned()
            .collect())
    }

    async fn list_active(&self) -> Result<Vec<Job>, PortError> {
        let lock = self.db.lock().unwrap();
        Ok(lock
            .jobs
            .values()
            .filter(|j| j.status() == &domain::job::JobStatus::Running)
            .cloned()
            .collect())
    }

    async fn list_recent(&self, limit: usize) -> Result<Vec<Job>, PortError> {
        let lock = self.db.lock().unwrap();
        let mut jobs: Vec<Job> = lock.jobs.values().cloned().collect();
        jobs.sort_by_key(|b| std::cmp::Reverse(*b.created_at()));
        Ok(jobs.into_iter().take(limit).collect())
    }
}
