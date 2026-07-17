use crate::error::PortError;
use crate::job_scheduler::ScheduledJob;
use async_trait::async_trait;
use domain::project::ProjectId;

#[async_trait]
pub trait JobQueryPort: Send + Sync {
    async fn list_jobs_snapshot(
        &self,
        project_id: &ProjectId,
    ) -> Result<Vec<ScheduledJob>, PortError>;
}
