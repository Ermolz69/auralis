use serde::{Deserialize, Serialize};

use crate::id::JobId;
use crate::progress::JobProgress;
use crate::stage::JobStage;
use domain::job::JobStatus;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_event_serialization() {
        let event = JobEvent {
            job_id: JobId("test-job-id".to_string()),
            project_id: Some("test-project-id".to_string()),
            status: JobStatus::Running,
            stage: Some(JobStage::FetchMetadata),
            progress: JobProgress { percent: 50 },
            message: Some("Fetching...".to_string()),
            error: None,
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(
            parsed.get("projectId").unwrap().as_str().unwrap(),
            "test-project-id"
        );
        assert_eq!(
            parsed.get("jobId").unwrap().as_str().unwrap(),
            "test-job-id"
        );
        assert_eq!(parsed.get("status").unwrap().as_str().unwrap(), "running");
        assert_eq!(
            parsed.get("stage").unwrap().as_str().unwrap(),
            "fetch_metadata"
        );
    }
}
