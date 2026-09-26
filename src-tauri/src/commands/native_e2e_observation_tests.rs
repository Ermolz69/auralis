use super::native_e2e_observation::is_start_observation;
use serde_json::{Value, json};

const RUN: &str = "0b33d519-6930-4469-a6c9-147f52ebe8c8";
const JOB: &str = "ef7d7629-4c6c-4d73-b0ae-6499eb83dff4";

fn checkpoint(record: &Value) -> String {
    format!("translation-concurrent-starts:{record}")
}

fn busy() -> Value {
    json!({"boundary": "initial_hash", "runId": RUN, "outcomes": [
        {"status": "rejected", "code": "BUSY"},
        {"status": "rejected", "code": "BUSY"}
    ]})
}

#[test]
fn accepts_only_declared_busy_and_accepted_observation_shapes() {
    assert!(is_start_observation(&checkpoint(&busy())));
    let accepted = json!({"boundary": "worker_preflight", "runId": RUN, "outcomes": [
        {"status": "fulfilled", "state": "accepted", "runId": RUN, "jobId": JOB},
        {"status": "fulfilled", "state": "accepted", "runId": RUN, "jobId": JOB}
    ]});
    assert!(is_start_observation(&checkpoint(&accepted)));
    for field in ["runId", "jobId"] {
        let mut record = accepted.clone();
        record["outcomes"][0][field] = json!("invalid-id");
        assert!(!is_start_observation(&checkpoint(&record)));
    }
}

#[test]
fn rejects_unbounded_multiline_malformed_or_unrecognised_observations() {
    for text in [
        "translation-concurrent-starts:{".to_owned(),
        checkpoint(&busy()) + "\n",
        checkpoint(&busy()) + "\r",
        checkpoint(&busy()) + &" ".repeat(1024),
        "other:".to_owned() + &busy().to_string(),
    ] {
        assert!(!is_start_observation(&text));
    }
    for (field, value) in [
        ("boundary", json!("other")),
        ("runId", json!("invalid")),
        ("outcomes", json!([])),
        ("extra", json!("unexpected")),
    ] {
        let mut record = busy();
        record[field] = value;
        assert!(!is_start_observation(&checkpoint(&record)));
    }
}

#[test]
fn rejects_other_errors_successes_and_extra_outcome_fields() {
    for (field, value) in [
        ("code", json!("CANCELLED")),
        ("status", json!("fulfilled")),
        ("message", json!("unexpected")),
    ] {
        let mut record = busy();
        record["outcomes"][0][field] = value;
        assert!(!is_start_observation(&checkpoint(&record)));
    }
}
