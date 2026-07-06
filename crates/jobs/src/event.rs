use serde::{Deserialize, Serialize};

use crate::id::JobId;
use crate::progress::JobProgress;
use crate::stage::JobStage;
use crate::status::JobStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobEvent {
    pub job_id: JobId,
    pub project_id: Option<String>,
    pub status: JobStatus,
    pub stage: Option<JobStage>,
    pub progress: JobProgress,
    pub message: Option<String>,
    pub error: Option<String>,
}
