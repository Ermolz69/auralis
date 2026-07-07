use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::id::JobId;
use crate::progress::JobProgress;
use crate::stage::JobStage;
use domain::job::JobStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: JobId,
    pub title: String,
    pub project_id: Option<String>,
    pub status: JobStatus,
    pub stage: Option<JobStage>,
    pub progress: JobProgress,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Job {
    pub fn new(title: String, project_id: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: JobId::new(),
            title,
            project_id,
            status: JobStatus::Pending,
            stage: None,
            progress: JobProgress::default(),
            error: None,
            created_at: now,
            updated_at: now,
        }
    }
}
