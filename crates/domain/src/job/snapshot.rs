use chrono::{DateTime, Utc};

use crate::dubbing::DubbingPipelineStage;
use crate::project::ProjectId;

use super::{JobError, JobId, JobKind, JobProgress, JobStatus};

pub struct JobSnapshot {
    pub id: JobId,
    pub project_id: ProjectId,
    pub kind: JobKind,
    pub status: JobStatus,
    pub stage: Option<DubbingPipelineStage>,
    pub progress: JobProgress,
    pub error: Option<JobError>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}


