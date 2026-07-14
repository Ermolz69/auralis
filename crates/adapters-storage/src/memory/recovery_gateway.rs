use async_trait::async_trait;
use domain::job::Job;
use domain::project::Project;
use ports::error::PortError;
use ports::recovery::{RecoverySnapshot, RecoveryStorage};

#[derive(Clone, Default)]
pub struct InMemoryRecoveryStorage;

impl InMemoryRecoveryStorage {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl RecoveryStorage for InMemoryRecoveryStorage {
    async fn load_snapshot(&self) -> Result<RecoverySnapshot, PortError> {
        // Dev-only fallback
        Ok(RecoverySnapshot {
            processing_projects: vec![],
            linked_jobs: vec![],
            active_jobs: vec![],
        })
    }

    async fn commit_interrupted_pair(&self, _project: Project, _job: Job) -> Result<(), PortError> {
        Ok(())
    }

    async fn commit_reconciled_project(&self, _project: Project) -> Result<(), PortError> {
        Ok(())
    }

    async fn commit_failed_project_no_job(&self, _project: Project) -> Result<(), PortError> {
        Ok(())
    }

    async fn commit_orphan_job(&self, _job: Job) -> Result<(), PortError> {
        Ok(())
    }
}
