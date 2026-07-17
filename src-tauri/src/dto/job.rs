#![allow(clippy::unwrap_used, clippy::expect_used)]
use chrono::{DateTime, Utc};
use ports::job_scheduler::ScheduledJob;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JobProgressDto {
    pub percent: u8,
    pub message: String,
    pub current_step: Option<String>,
    pub processed_items: Option<u64>,
    pub total_items: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobDto {
    pub id: String,
    pub project_id: Option<String>,
    pub title: String,
    pub status: String,
    pub stage: Option<String>,
    pub progress: JobProgressDto,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&ScheduledJob> for JobDto {
    fn from(job: &ScheduledJob) -> Self {
        let stage = job
            .stage
            .as_ref()
            .map(adapters_tauri::job_event_mapper::map_stage);

        Self {
            id: job.id.to_string(),
            project_id: job.project_id.as_ref().map(|id| id.to_string()),
            title: job.title.clone(),
            status: adapters_tauri::job_event_mapper::map_status(&job.status),
            stage,
            progress: JobProgressDto {
                percent: job.progress.percent,
                message: job.progress.message.clone(),
                current_step: job.progress.current_step.clone(),
                processed_items: job.progress.processed_items,
                total_items: job.progress.total_items,
            },
            error: job.error.clone(),
            created_at: job.created_at,
            updated_at: job.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::dubbing::DubbingPipelineStage;
    use domain::job::{JobId, JobProgress, JobStatus};
    use domain::project::ProjectId;
    use ports::job_scheduler::ScheduledJob;

    #[test]
    fn test_job_dto_serialization_contract() {
        let job = ScheduledJob {
            id: JobId::new(),
            revision: 1,
            project_id: Some(ProjectId::new()),
            title: "Test Job".to_string(),
            status: JobStatus::Pending,
            stage: Some(DubbingPipelineStage::MuxAudioTrack),
            progress: JobProgress {
                percent: 100,
                message: "Muxing done".to_string(),
                current_step: Some("mux".to_string()),
                processed_items: Some(2),
                total_items: Some(2),
            },
            error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let dto = JobDto::from(&job);
        let serialized = serde_json::to_value(&dto).unwrap();

        assert_eq!(serialized["status"], "pending");
        assert_eq!(serialized["stage"], "muxAudioTrack");
        assert_eq!(serialized["progress"]["message"], "Muxing done");
    }
}
