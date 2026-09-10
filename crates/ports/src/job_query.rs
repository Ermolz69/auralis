use crate::error::PortError;
use crate::job_scheduler::ScheduledJob;
use async_trait::async_trait;
use domain::job::JobId;
use domain::project::ProjectId;

pub const MAX_JOB_HISTORY_PAGE_SIZE: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobHistoryCursor {
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub job_id: JobId,
}

#[derive(Debug, Clone)]
pub struct JobHistoryPage {
    pub jobs: Vec<ScheduledJob>,
    pub next_cursor: Option<JobHistoryCursor>,
}

#[async_trait]
pub trait JobQueryPort: Send + Sync {
    async fn list_jobs_snapshot(
        &self,
        project_id: &ProjectId,
    ) -> Result<Vec<ScheduledJob>, PortError>;

    async fn list_job_history_page(
        &self,
        cursor: Option<&JobHistoryCursor>,
        limit: usize,
    ) -> Result<JobHistoryPage, PortError>;
}
