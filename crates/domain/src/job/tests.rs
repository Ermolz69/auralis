use super::*;
use crate::dubbing::DubbingPipelineStage;
use crate::project::ProjectId;

#[test]
fn test_job_transitions() {
    let project_id = ProjectId::new();
    let mut job = Job::new(project_id, JobKind::Dubbing);

    assert_eq!(job.status(), &JobStatus::Pending);

    // Start job
    assert!(job.start().is_ok());
    assert_eq!(job.status(), &JobStatus::Running);
    assert!(job.started_at().is_some());

    // Update progress
    let progress = JobProgress {
        percent: 50,
        message: "Halfway there".to_string(),
        current_step: Some("Translating".to_string()),
        processed_items: Some(5),
        total_items: Some(10),
    };
    assert!(job.update_progress(progress).is_ok());

    // Update stage
    assert!(job.update_stage(DubbingPipelineStage::TranslateTranscript).is_ok());

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
    let mut job = Job::new(project_id, JobKind::Dubbing);

    job.start().unwrap();

    let error = JobError::new(
        "ERR_DOWNLOAD",
        "Failed to download media",
        true,
    );

    assert!(job.mark_failed(error).is_ok());
    assert_eq!(job.status(), &JobStatus::Failed);
    assert!(job.error().is_some());
}
