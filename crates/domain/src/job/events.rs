use crate::dubbing::DubbingPipelineStage;
use crate::job::{Job, JobId, JobProgress, JobError};

#[derive(Debug, Clone, PartialEq)]
pub enum JobEvent {
    Created(Job),
    ProgressUpdated {
        job_id: JobId,
        stage: Option<DubbingPipelineStage>,
        progress: JobProgress,
    },
    Completed {
        job_id: JobId,
    },
    Failed {
        job_id: JobId,
        error: JobError,
    },
    Cancelled {
        job_id: JobId,
    },
}
