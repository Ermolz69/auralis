#![allow(clippy::unwrap_used)]
use crate::error::DomainError;
use crate::job::{JobId, TerminalOutcome};
use crate::media::MediaSource;
use crate::project::entity::{LanguageCode, Project};
use crate::project::status::{ProjectStatus, TerminalTransitionResult};

#[test]
fn test_project_transitions() {
    let mut project = Project::new("Test Video".to_string());
    assert_eq!(project.status(), &ProjectStatus::Draft);

    // Cannot start processing from Draft
    let job_id = JobId::new();
    assert!(project.start_processing(job_id.clone()).is_err());

    // Import source
    let source = MediaSource::RemoteUrl {
        url: "https://example.com/video.mp4".to_string(),
    };
    assert!(project.import_source(source, None).is_ok());
    assert_eq!(project.status(), &ProjectStatus::SourceImported);

    // Mark ready
    assert!(project.mark_ready_for_processing().is_ok());
    assert_eq!(project.status(), &ProjectStatus::ReadyForProcessing);

    // Start processing
    assert!(project.start_processing(job_id.clone()).is_ok());
    assert_eq!(project.status(), &ProjectStatus::Processing);
    assert_eq!(project.active_job_id(), Some(&job_id));

    // Wrong job cannot complete
    let wrong_job_id = JobId::new();
    let result = project
        .apply_terminal_transition(&wrong_job_id, TerminalOutcome::Completed)
        .unwrap();
    assert_eq!(result, TerminalTransitionResult::IgnoredStale);
    assert_eq!(project.status(), &ProjectStatus::Processing);

    // Complete
    let result = project
        .apply_terminal_transition(&job_id, TerminalOutcome::Completed)
        .unwrap();
    assert_eq!(
        result,
        TerminalTransitionResult::Applied {
            transcript_ready: false
        }
    );
    assert_eq!(project.status(), &ProjectStatus::Completed);
    assert_eq!(project.last_terminal_job_id().unwrap(), &job_id);

    // Cannot cancel completed project
    let result = project
        .apply_terminal_transition(&job_id, TerminalOutcome::Cancelled)
        .unwrap();
    assert_eq!(result, TerminalTransitionResult::AlreadyApplied);
}

#[test]
fn test_project_fail_and_retry() {
    let mut project = Project::new("Retry Test".to_string());
    let source = MediaSource::RemoteUrl {
        url: "https://example.com/video.mp4".to_string(),
    };
    project.import_source(source, None).unwrap();
    project.mark_ready_for_processing().unwrap();

    let job_id1 = JobId::new();
    project.start_processing(job_id1.clone()).unwrap();

    // Fail
    let result = project
        .apply_terminal_transition(&job_id1, TerminalOutcome::Failed)
        .unwrap();
    assert_eq!(
        result,
        TerminalTransitionResult::Applied {
            transcript_ready: false
        }
    );
    assert_eq!(project.status(), &ProjectStatus::Failed);

    // Retry (start processing again from failed state)
    let job_id2 = JobId::new();
    assert!(project.start_processing(job_id2.clone()).is_ok());
    assert_eq!(project.status(), &ProjectStatus::Processing);
    assert_eq!(project.active_job_id(), Some(&job_id2));

    // Stale failure from first job is ignored
    let result = project
        .apply_terminal_transition(&job_id1, TerminalOutcome::Failed)
        .unwrap();
    assert_eq!(result, TerminalTransitionResult::AlreadyApplied);
    assert_eq!(project.status(), &ProjectStatus::Processing);
}

#[test]
fn test_from_snapshot_rejects_empty_title() {
    let mut snapshot = Project::new("Valid Title".to_string()).to_snapshot();
    snapshot.title = "   ".to_string(); // Empty/whitespace title

    let result = Project::from_snapshot(snapshot);
    assert!(matches!(result, Err(DomainError::ValidationError(_))));
}

#[test]
fn test_from_snapshot_rejects_empty_language_code() {
    let mut snapshot = Project::new("Valid Title".to_string()).to_snapshot();
    snapshot.source_language = Some(LanguageCode("   ".to_string()));

    let result = Project::from_snapshot(snapshot);
    assert!(matches!(result, Err(DomainError::ValidationError(_))));
}

#[test]
fn test_from_snapshot_rejects_active_status_without_source() {
    let mut snapshot = Project::new("Valid Title".to_string()).to_snapshot();
    snapshot.source = None;

    // Try states that require a source
    let active_states = vec![
        ProjectStatus::SourceImported,
        ProjectStatus::ReadyForProcessing,
        ProjectStatus::Processing,
        ProjectStatus::Completed,
    ];

    for state in active_states {
        snapshot.status = state;
        let result = Project::from_snapshot(snapshot.clone());
        assert!(matches!(result, Err(DomainError::ValidationError(_))));
    }
}
