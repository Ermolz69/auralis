use crate::job_event_dto::{JobEventDto, JobProgressDto};
use ports::job_scheduler::JobLifecycleEvent;

pub struct JobEventDtoMapper;

impl JobEventDtoMapper {
    pub fn map(event: &JobLifecycleEvent) -> JobEventDto {
        let status = match &event.status {
            domain::job::JobStatus::Pending => "pending".to_string(),
            domain::job::JobStatus::Running => "running".to_string(),
            domain::job::JobStatus::Completed => "completed".to_string(),
            domain::job::JobStatus::Failed => "failed".to_string(),
            domain::job::JobStatus::Cancelled => "cancelled".to_string(),
        };

        let stage = event.stage.as_ref().map(|s| match s {
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

        JobEventDto {
            job_id: event.job_id.to_string(),
            project_id: event.project_id.as_ref().map(|id| id.to_string()),
            status,
            stage,
            progress: JobProgressDto {
                percent: event.progress.percent,
                message: event.progress.message.clone(),
                current_step: event.progress.current_step.clone(),
                processed_items: event.progress.processed_items,
                total_items: event.progress.total_items,
            },
            error: event.error.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::dubbing::DubbingPipelineStage;
    use domain::job::{JobId, JobProgress, JobStatus};
    use domain::project::ProjectId;
    use serde_json::json;

    #[test]
    fn test_dto_serialization_contract() {
        let event = JobLifecycleEvent {
            job_id: JobId::new(),
            project_id: Some(ProjectId::new()),
            status: JobStatus::Running,
            stage: Some(DubbingPipelineStage::DownloadMedia),
            progress: JobProgress {
                percent: 50,
                message: "Downloading".to_string(),
                current_step: Some("video.mp4".to_string()),
                processed_items: Some(1),
                total_items: Some(2),
            },
            error: None,
        };

        let dto = JobEventDtoMapper::map(&event);
        let serialized = serde_json::to_value(&dto).unwrap();

        assert_eq!(
            serialized,
            json!({
                "jobId": event.job_id.to_string(),
                "projectId": event.project_id.unwrap().to_string(),
                "status": "running",
                "stage": "downloadMedia",
                "progress": {
                    "percent": 50,
                    "message": "Downloading",
                    "currentStep": "video.mp4",
                    "processedItems": 1,
                    "totalItems": 2
                },
                "error": null
            })
        );
    }

    #[test]
    fn test_dto_serialization_none_handling() {
        let event = JobLifecycleEvent {
            job_id: JobId::new(),
            project_id: None,
            status: JobStatus::Pending,
            stage: None,
            progress: JobProgress::initializing(),
            error: Some("Fail".to_string()),
        };

        let dto = JobEventDtoMapper::map(&event);
        let serialized = serde_json::to_value(&dto).unwrap();

        assert_eq!(
            serialized,
            json!({
                "jobId": event.job_id.to_string(),
                "projectId": null,
                "status": "pending",
                "stage": null,
                "progress": {
                    "percent": 0,
                    "message": "Initializing...",
                    "currentStep": null,
                    "processedItems": null,
                    "totalItems": null
                },
                "error": "Fail"
            })
        );
    }
}
