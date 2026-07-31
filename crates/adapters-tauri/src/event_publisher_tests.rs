use crate::event_publisher::{EVENT_JOB_EVENT, EVENT_JOB_EVENTS_INVALIDATED};

#[test]
fn test_event_names_contract() {
    assert_eq!(EVENT_JOB_EVENT, "job-event");
    assert_eq!(EVENT_JOB_EVENTS_INVALIDATED, "job-events-invalidated");
}
