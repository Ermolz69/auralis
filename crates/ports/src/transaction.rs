use async_trait::async_trait;

use crate::error::PortError;
use domain::job::Job;
use domain::media::{Artifact, ArtifactId};
use domain::outbox::OutboxMessage;
use domain::project::Project;

#[derive(Default, Debug, Clone)]
pub struct UnitOfWorkData {
    pub jobs_to_save: Vec<Job>,
    pub projects_to_save: Vec<Project>,
    pub artifacts_to_add: Vec<(domain::project::ProjectId, Artifact)>,
    pub artifacts_to_delete: Vec<ArtifactId>,
    pub outbox_messages: Vec<OutboxMessage>,
}

impl UnitOfWorkData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn save_job(mut self, job: Job) -> Self {
        self.jobs_to_save.push(job);
        self
    }

    pub fn save_project(mut self, project: Project) -> Self {
        self.projects_to_save.push(project);
        self
    }

    pub fn add_artifact(mut self, project_id: domain::project::ProjectId, artifact: Artifact) -> Self {
        self.artifacts_to_add.push((project_id, artifact));
        self
    }

    pub fn delete_artifact(mut self, artifact_id: ArtifactId) -> Self {
        self.artifacts_to_delete.push(artifact_id);
        self
    }

    pub fn add_outbox_message(mut self, message: OutboxMessage) -> Self {
        self.outbox_messages.push(message);
        self
    }
}

#[async_trait]
pub trait TransactionGateway: Send + Sync {
    async fn execute(&self, data: UnitOfWorkData) -> Result<(), PortError>;
}
