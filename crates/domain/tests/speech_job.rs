use domain::job::{Job, JobKind};
use domain::project::ProjectId;

#[test]
fn speech_progress_tracks_only_saved_chunks_and_stays_below_completion() {
    let mut job = Job::new(ProjectId::new(), "Speech".into(), JobKind::Speech);
    assert!(job.start().is_ok());
    assert!(job.report_speech_progress(1, 3).is_ok());
    assert_eq!(job.progress().processed_items, Some(1));
    assert_eq!(job.progress().total_items, Some(3));
    assert!(job.progress().percent < 100);
    assert!(job.report_speech_progress(0, 3).is_err());
    assert!(job.report_speech_progress(2, 4).is_err());
    assert!(job.report_speech_progress(3, 3).is_ok());
    assert_eq!(job.progress().percent, 99);
    assert!(job.mark_completed().is_ok());
    assert_eq!(job.progress().percent, 100);
}

#[test]
fn another_job_kind_cannot_accept_speech_checkpoints() {
    let mut job = Job::new(ProjectId::new(), "Translate".into(), JobKind::Translation);
    assert!(job.start().is_ok());
    assert!(job.report_speech_progress(1, 2).is_err());
}
