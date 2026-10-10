use adapters_tts::{WorkerIdentity, WorkerProtocol};
use serde_json::{Value, json};

fn identity() -> WorkerIdentity {
    WorkerIdentity {
        request_id: "request-1".into(),
        project_id: "project-1".into(),
        run_id: "run-1".into(),
        attempt_id: "attempt-1".into(),
    }
}

fn event(sequence: u64) -> Vec<u8> {
    let mut value = json!({
        "protocol_version": 1,
        "request_id": "request-1",
        "project_id": "project-1",
        "run_id": "run-1",
        "attempt_id": "attempt-1",
        "event_sequence": sequence,
        "event_kind": "saved_checkpoint",
        "payload": {"chunk_id": "chunk-1"},
    })
    .to_string()
    .into_bytes();
    value.push(b'\n');
    value
}

#[test]
fn accepts_only_ordered_events_for_the_active_attempt() {
    let mut protocol = WorkerProtocol::default();
    assert!(
        matches!(protocol.parse(&event(1), &identity()), Ok(value) if value.event_kind == "saved_checkpoint")
    );
    assert!(protocol.parse(&event(1), &identity()).is_err());
    assert!(protocol.parse(&event(3), &identity()).is_err());
    assert!(
        matches!(protocol.parse(&event(2), &identity()), Ok(value) if value.event_kind == "saved_checkpoint")
    );
}

#[test]
fn rejects_stale_identity_and_malformed_frames() {
    let mut protocol = WorkerProtocol::default();
    let mut stale: Value = json!({
        "protocol_version": 1,
        "request_id": "request-1",
        "project_id": "project-1",
        "run_id": "run-1",
        "attempt_id": "attempt-1",
        "event_sequence": 1,
        "event_kind": "saved_checkpoint",
        "payload": {"chunk_id": "chunk-1"},
    });
    stale["attempt_id"] = json!("replaced-attempt");
    let mut stale = stale.to_string().into_bytes();
    stale.push(b'\n');
    assert!(protocol.parse(&stale, &identity()).is_err());
    assert!(
        protocol
            .parse(&event(1)[..event(1).len() - 1], &identity())
            .is_err()
    );
    let mut extra = event(1);
    extra.extend_from_slice(&event(2));
    assert!(protocol.parse(&extra, &identity()).is_err());
    assert!(
        matches!(protocol.parse(&event(1), &identity()), Ok(value) if value.event_kind == "saved_checkpoint")
    );
}

#[test]
fn permits_the_explicit_cancel_request_identity_in_one_attempt() {
    let execute = identity();
    let cancel = WorkerIdentity {
        request_id: "cancel-1".into(),
        ..execute.clone()
    };
    let mut protocol = WorkerProtocol::default();
    let mut frame = event(1);
    let text = String::from_utf8_lossy(&frame).replace("request-1", "cancel-1");
    frame = text.into_bytes();
    assert!(matches!(
        protocol.parse_one_of(&frame, &[&execute, &cancel]),
        Ok(value) if value.event_kind == "saved_checkpoint"
    ));
    assert!(protocol.parse(&event(2), &execute).is_ok());
}
