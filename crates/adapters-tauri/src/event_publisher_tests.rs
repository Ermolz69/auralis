use crate::event_publisher::{
    EVENT_JOB_EVENT, EVENT_JOB_EVENTS_INVALIDATED, EVENT_PROJECT_UPDATED, EVENT_TRANSCRIPT_READY,
};

#[test]
fn test_event_names_contract() {
    assert_eq!(EVENT_JOB_EVENT, "job-event");
    assert_eq!(EVENT_JOB_EVENTS_INVALIDATED, "job-events-invalidated");
    assert_eq!(EVENT_PROJECT_UPDATED, "project-updated");
    assert_eq!(EVENT_TRANSCRIPT_READY, "transcript-ready");
}
