use async_trait::async_trait;
use domain::job::Job;
use domain::project::Project;

use crate::error::PortError;

pub struct RecoverySnapshot {
    pub processing_projects: Vec<Project>,
    pub linked_jobs: Vec<Job>,
    pub active_jobs: Vec<Job>,
}

#[async_trait]
pub trait RecoveryStorage: Send + Sync {
    async fn load_snapshot(&self) -> Result<RecoverySnapshot, PortError>;

    async fn commit_interrupted_pair(&self, project: Project, job: Job) -> Result<(), PortError>;

    async fn commit_reconciled_project(&self, project: Project) -> Result<(), PortError>;

    async fn commit_failed_project_no_job(&self, project: Project) -> Result<(), PortError>;

    async fn commit_orphan_job(&self, job: Job) -> Result<(), PortError>;
}
