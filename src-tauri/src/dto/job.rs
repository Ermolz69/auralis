use chrono::{DateTime, Utc};
use domain::job::JobStatus;
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
        let stage = job.stage.as_ref().map(|s| match s {
            domain::dubbing::DubbingPipelineStage::ValidateSource => "validateSource".to_string(),
            domain::dubbing::DubbingPipelineStage::InspectSubtitles => {
                "inspectSubtitles".to_string()
            }
            domain::dubbing::DubbingPipelineStage::FetchMetadata => "fetchMetadata".to_string(),
            domain::dubbing::DubbingPipelineStage::DownloadMedia => "downloadMedia".to_string(),
            domain::dubbing::DubbingPipelineStage::ExtractOrGenerateTranscript => {
                "extractOrGenerateTranscript".to_string()
            }
            domain::dubbing::DubbingPipelineStage::SegmentTranscript => {
                "segmentTranscript".to_string()
            }
            domain::dubbing::DubbingPipelineStage::TranslateTranscript => {
                "translateTranscript".to_string()
            }
            domain::dubbing::DubbingPipelineStage::PrepareDubbingScript => {
                "prepareDubbingScript".to_string()
            }
            domain::dubbing::DubbingPipelineStage::SynthesizeSegments => {
                "synthesizeSegments".to_string()
            }
            domain::dubbing::DubbingPipelineStage::PostprocessAudio => {
                "postprocessAudio".to_string()
            }
            domain::dubbing::DubbingPipelineStage::MuxAudioTrack => "muxAudioTrack".to_string(),
            domain::dubbing::DubbingPipelineStage::ExportResult => "exportResult".to_string(),
        });

        Self {
            id: job.id.to_string(),
            project_id: job.project_id.as_ref().map(|id| id.to_string()),
            title: job.title.clone(),
            status: match job.status {
                JobStatus::Pending => "pending".into(),
                JobStatus::Running => "running".into(),
                JobStatus::Completed => "completed".into(),
                JobStatus::Failed => "failed".into(),
                JobStatus::Cancelled => "cancelled".into(),
            },
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
