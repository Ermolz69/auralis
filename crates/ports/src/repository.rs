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
    async fn list_recent(&self, limit: usize) -> Result<Vec<Job>, PortError>;
}

use domain::outbox::{OutboxMessage, OutboxMessageId};

#[async_trait]
pub trait OutboxRepository: Send + Sync {
    async fn fetch_pending(&self, limit: usize) -> Result<Vec<OutboxMessage>, PortError>;
    async fn mark_processing(&self, id: &OutboxMessageId, locked_by: &str)
    -> Result<(), PortError>;
    async fn mark_done(&self, id: &OutboxMessageId) -> Result<(), PortError>;
    async fn mark_failed(&self, id: &OutboxMessageId, error: &str) -> Result<(), PortError>;
}
use std::sync::Arc;

#[async_trait]
impl<T> ProjectRepository for Arc<T>
where
    T: ProjectRepository + ?Sized,
{
    async fn create(&self, project: Project) -> Result<Project, PortError> {
        (**self).create(project).await
    }

    async fn get(&self, id: &ProjectId) -> Result<Option<Project>, PortError> {
        (**self).get(id).await
    }

    async fn save(&self, project: &Project) -> Result<(), PortError> {
        (**self).save(project).await
    }

    async fn list(&self) -> Result<Vec<Project>, PortError> {
        (**self).list().await
    }

    async fn delete(&self, id: &ProjectId) -> Result<(), PortError> {
        (**self).delete(id).await
    }
}

#[async_trait]
impl<T> OutboxRepository for Arc<T>
where
    T: OutboxRepository + ?Sized,
{
    async fn fetch_pending(&self, limit: usize) -> Result<Vec<OutboxMessage>, PortError> {
        (**self).fetch_pending(limit).await
    }

    async fn mark_processing(
        &self,
        id: &OutboxMessageId,
        locked_by: &str,
    ) -> Result<(), PortError> {
        (**self).mark_processing(id, locked_by).await
    }

    async fn mark_done(&self, id: &OutboxMessageId) -> Result<(), PortError> {
        (**self).mark_done(id).await
    }

    async fn mark_failed(&self, id: &OutboxMessageId, error: &str) -> Result<(), PortError> {
        (**self).mark_failed(id, error).await
    }
}
