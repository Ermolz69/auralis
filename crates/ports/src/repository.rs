use async_trait::async_trait;

use crate::error::PortError;
use domain::job::{Job, JobId};
use domain::project::{Project, ProjectId};

#[async_trait]
pub trait ProjectRepository: Send + Sync {
    async fn create(&self, project: Project) -> Result<Project, PortError>;
    async fn get(&self, id: &ProjectId) -> Result<Option<Project>, PortError>;
    async fn save(&self, project: &Project) -> Result<(), PortError>;
    async fn list(&self) -> Result<Vec<Project>, PortError>;
    async fn delete(&self, id: &ProjectId) -> Result<(), PortError>;
}

#[async_trait]
pub trait JobRepository: Send + Sync {
    async fn create(&self, job: Job) -> Result<Job, PortError>;
    async fn get(&self, id: &JobId) -> Result<Option<Job>, PortError>;
    async fn save(&self, job: &Job) -> Result<(), PortError>;
    async fn list_by_project(&self, project_id: &ProjectId) -> Result<Vec<Job>, PortError>;
    async fn list_active(&self) -> Result<Vec<Job>, PortError>;
}
