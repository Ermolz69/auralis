use super::*;
use crate::dubbing::DubbingPipelineStage;
use crate::project::ProjectId;

#[test]
fn test_job_transitions() {
    let project_id = ProjectId::new();
    let mut job = Job::new(project_id, "Test Job".to_string(), JobKind::Dubbing);

    assert_eq!(job.status(), &JobStatus::Pending);

    // Start job
    assert!(job.start().is_ok());
    assert_eq!(job.status(), &JobStatus::Running);
    assert!(job.started_at().is_some());

    // Advance job
    let progress = JobProgress {
        percent: 50,
        message: "Halfway there".to_string(),
        current_step: Some("Translating".to_string()),
        processed_items: Some(5),
        total_items: Some(10),
    };
    assert!(
        job.advance(DubbingPipelineStage::TranslateTranscript, progress)
            .is_ok()
    );

    // Complete job
    assert!(job.mark_completed().is_ok());
    assert_eq!(job.status(), &JobStatus::Completed);
    assert_eq!(job.progress().percent, 100);
    assert!(job.finished_at().is_some());

    // Cannot cancel completed job
    assert!(job.cancel().is_err());
}

#[test]
fn test_job_failure() {
    let project_id = ProjectId::new();
    let mut job = Job::new(project_id, "Test Job".to_string(), JobKind::Dubbing);

    job.start().unwrap();

    let error = JobError::new("ERR_DOWNLOAD", "Failed to download media", true);

    assert!(job.mark_failed(error).is_ok());
    assert_eq!(job.status(), &JobStatus::Failed);
    assert!(job.error().is_some());
}

#[test]
fn test_job_advance_invalid_progress() {
    let project_id = ProjectId::new();
    let mut job = Job::new(project_id, "Test Job".to_string(), JobKind::Dubbing);
    job.start().unwrap();

    let initial_updated_at = *job.updated_at();

    let invalid_progress = JobProgress {
        percent: 150, // > 100
        message: "Too far".to_string(),
        current_step: None,
        processed_items: None,
        total_items: None,
    };

    let result = job.advance(DubbingPipelineStage::FetchMetadata, invalid_progress);
    assert!(result.is_err());

    // State must remain unchanged
    assert_eq!(job.stage(), None);
    assert_eq!(job.progress().percent, 0);
    assert_eq!(job.updated_at(), &initial_updated_at);
}

#[test]
fn test_job_advance_non_running() {
    let project_id = ProjectId::new();
    let mut job = Job::new(project_id, "Test Job".to_string(), JobKind::Dubbing);

    let progress = JobProgress::initializing();

    let result = job.advance(DubbingPipelineStage::FetchMetadata, progress);
    assert!(result.is_err());
    assert_eq!(job.stage(), None);
}

#[test]
fn test_job_cancel_idempotence() {
    let project_id = ProjectId::new();
    let mut job = Job::new(project_id, "Test Job".to_string(), JobKind::Dubbing);

    assert!(job.cancel().is_ok());
    assert_eq!(job.status(), &JobStatus::Cancelled);

    // Repeat cancellation should be idempotent success
    assert!(job.cancel().is_ok());
    assert_eq!(job.status(), &JobStatus::Cancelled);
}
