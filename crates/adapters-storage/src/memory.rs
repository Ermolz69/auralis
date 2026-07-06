use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;

use domain::job::{Job, JobId};
use domain::project::{Project, ProjectId};
use ports::error::PortError;
use ports::repository::{JobRepository, ProjectRepository};

#[derive(Clone)]
pub struct InMemoryProjectRepository {
    pub projects: Arc<Mutex<HashMap<ProjectId, Project>>>,
}

impl InMemoryProjectRepository {
    pub fn new() -> Self {
        Self {
            projects: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for InMemoryProjectRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProjectRepository for InMemoryProjectRepository {
    async fn create(&self, project: Project) -> Result<Project, PortError> {
        let mut lock = self.projects.lock().unwrap();
        lock.insert(project.id().clone(), project.clone());
        Ok(project)
    }

    async fn get(&self, id: &ProjectId) -> Result<Option<Project>, PortError> {
        let lock = self.projects.lock().unwrap();
        Ok(lock.get(id).cloned())
    }

    async fn save(&self, project: &Project) -> Result<(), PortError> {
        let mut lock = self.projects.lock().unwrap();
        lock.insert(project.id().clone(), project.clone());
        Ok(())
    }

    async fn list(&self) -> Result<Vec<Project>, PortError> {
        let lock = self.projects.lock().unwrap();
        Ok(lock.values().cloned().collect())
    }

    async fn delete(&self, id: &ProjectId) -> Result<(), PortError> {
        let mut lock = self.projects.lock().unwrap();
        lock.remove(id);
        Ok(())
    }
}

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
        Ok(lock.values().filter(|j| j.project_id() == project_id).cloned().collect())
    }

    async fn list_active(&self) -> Result<Vec<Job>, PortError> {
        let lock = self.jobs.lock().unwrap();
        Ok(lock.values().filter(|j| j.status() == &domain::job::JobStatus::Running).cloned().collect())
    }
}
