use crate::dto::job::{JobDto, JobProgressDto};
use crate::dto::job_event::{JobEventDto, JobLifecycleEventKindDto};
use domain::dubbing::DubbingPipelineStage;
use domain::job::JobStatus;
use ports::job_scheduler::{JobLifecycleEvent, JobLifecycleEventKind, ScheduledJob};

#[derive(Debug, Clone, thiserror::Error)]
pub enum JobDtoMappingError {
    #[error("Invalid revision {0}. Must be between 1 and 9007199254740991")]
    InvalidRevision(u64),
}

pub fn map_status(status: &JobStatus) -> String {
    match status {
        JobStatus::Pending => "pending".to_string(),
        JobStatus::Running => "running".to_string(),
        JobStatus::Completed => "completed".to_string(),
        JobStatus::Failed => "failed".to_string(),
        JobStatus::Cancelled => "cancelled".to_string(),
    }
}

pub fn map_stage(stage: &DubbingPipelineStage) -> String {
    match stage {
        DubbingPipelineStage::ValidateSource => "validateSource".to_string(),
        DubbingPipelineStage::InspectSubtitles => "inspectSubtitles".to_string(),
        DubbingPipelineStage::FetchMetadata => "fetchMetadata".to_string(),
        DubbingPipelineStage::DownloadMedia => "downloadMedia".to_string(),
        DubbingPipelineStage::ExtractOrGenerateTranscript => {
            "extractOrGenerateTranscript".to_string()
        }
        DubbingPipelineStage::SegmentTranscript => "segmentTranscript".to_string(),
        DubbingPipelineStage::TranslateTranscript => "translateTranscript".to_string(),
        DubbingPipelineStage::PrepareDubbingScript => "prepareDubbingScript".to_string(),
        DubbingPipelineStage::SynthesizeSegments => "synthesizeSegments".to_string(),
        DubbingPipelineStage::PostprocessAudio => "postprocessAudio".to_string(),
        DubbingPipelineStage::MuxAudioTrack => "muxAudioTrack".to_string(),
        DubbingPipelineStage::ExportResult => "exportResult".to_string(),
    }
}

pub fn map_kind(kind: &JobLifecycleEventKind) -> JobLifecycleEventKindDto {
    match kind {
        JobLifecycleEventKind::Created => JobLifecycleEventKindDto::Created,
        JobLifecycleEventKind::Started => JobLifecycleEventKindDto::Started,
        JobLifecycleEventKind::Progressed => JobLifecycleEventKindDto::Progressed,
        JobLifecycleEventKind::Completed => JobLifecycleEventKindDto::Completed,
        JobLifecycleEventKind::Failed => JobLifecycleEventKindDto::Failed,
        JobLifecycleEventKind::Cancelled => JobLifecycleEventKindDto::Cancelled,
    }
}

pub fn map_job_dto(job: &ScheduledJob) -> Result<JobDto, JobDtoMappingError> {
    if job.revision < 1 || job.revision > 9_007_199_254_740_991 {
        return Err(JobDtoMappingError::InvalidRevision(job.revision));
    }

    Ok(JobDto {
        id: job.id.to_string(),
        revision: job.revision,
        project_id: job.project_id.as_ref().map(|id| id.to_string()),
        title: job.title.clone(),
        status: map_status(&job.status),
        stage: job.stage.as_ref().map(map_stage),
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
    })
}

pub fn map_job_event_dto(event: &JobLifecycleEvent) -> Result<JobEventDto, JobDtoMappingError> {
    Ok(JobEventDto {
        kind: map_kind(&event.kind),
        job: map_job_dto(&event.job)?,
    })
}
