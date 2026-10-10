use domain::job::{Job, JobKind};
use domain::project::ProjectId;

#[test]
fn speech_progress_tracks_only_saved_chunks_and_stays_below_completion() {
    let mut job = Job::new(ProjectId::new(), "Speech".into(), JobKind::Speech);
    job.start().expect("pending speech job starts");
    job.report_speech_progress(1, 3)
        .expect("saved checkpoint is accepted");
    assert_eq!(job.progress().processed_items, Some(1));
    assert_eq!(job.progress().total_items, Some(3));
    assert!(job.progress().percent < 100);
    assert!(job.report_speech_progress(0, 3).is_err());
    assert!(job.report_speech_progress(2, 4).is_err());
    job.report_speech_progress(3, 3)
        .expect("all chunks can be saved");
    assert_eq!(job.progress().percent, 99);
    job.mark_completed().expect("publication completes the job");
    assert_eq!(job.progress().percent, 100);
}

#[test]
fn another_job_kind_cannot_accept_speech_checkpoints() {
    let mut job = Job::new(ProjectId::new(), "Translate".into(), JobKind::Translation);
    job.start().expect("pending translation job starts");
    assert!(job.report_speech_progress(1, 2).is_err());
}
