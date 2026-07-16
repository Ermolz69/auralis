use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::dubbing::DubbingPipelineStage;
use domain::job::{JobId, JobProgress, JobStatus};
use domain::project::ProjectId;
use serde::{Deserialize, Serialize};

use crate::error::PortError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartDubbingJobRequest {
    pub project_id: Option<ProjectId>,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledJob {
    pub id: JobId,
    pub revision: u64,
    pub project_id: Option<ProjectId>,
    pub title: String,
    pub status: JobStatus,
    pub stage: Option<DubbingPipelineStage>,
    pub progress: JobProgress,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[async_trait]
pub trait JobSchedulerPort: Send + Sync {
    async fn start_dubbing_job(
        &self,
        request: StartDubbingJobRequest,
    ) -> Result<ScheduledJob, PortError>;

    async fn enqueue_existing_job(&self, job_id: &JobId) -> Result<ScheduledJob, PortError>;

    async fn cancel_job(&self, job_id: &JobId) -> Result<ScheduledJob, PortError>;

    async fn get_job(&self, job_id: &JobId) -> Result<Option<ScheduledJob>, PortError>;

    async fn list_jobs(&self) -> Result<Vec<ScheduledJob>, PortError>;

    async fn list_jobs_snapshot(
        &self,
        project_id: &ProjectId,
    ) -> Result<Vec<ScheduledJob>, PortError>;

    async fn update_job_stage(
        &self,
        job_id: &JobId,
        stage: DubbingPipelineStage,
        progress: JobProgress,
    ) -> Result<ScheduledJob, PortError>;

    async fn complete_job(&self, job_id: &JobId) -> Result<ScheduledJob, PortError>;

    async fn fail_job(
        &self,
        job_id: &JobId,
        code: String,
        message: String,
        retryable: bool,
    ) -> Result<ScheduledJob, PortError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum JobLifecycleEventKind {
    Created,
    Started,
    Progressed,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobLifecycleEvent {
    pub kind: JobLifecycleEventKind,
    pub job: ScheduledJob,
}
