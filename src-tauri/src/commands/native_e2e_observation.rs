use serde_json::Value;

const PREFIX: &str = "translation-concurrent-starts:";
const MAX_CHECKPOINT_BYTES: usize = 1024;
const CONCURRENT_START_COUNT: usize = 2;

pub(super) fn is_start_observation(checkpoint: &str) -> bool {
    if checkpoint.len() > MAX_CHECKPOINT_BYTES || checkpoint.contains(['\n', '\r']) {
        return false;
    }
    let Some(payload) = checkpoint.strip_prefix(PREFIX) else {
        return false;
    };
    let Ok(record) = serde_json::from_str::<Value>(payload) else {
        return false;
    };
    if record.as_object().map(|object| object.len()) != Some(3) {
        return false;
    }
    let Some(run_id) = record["runId"].as_str() else {
        return false;
    };
    if uuid::Uuid::parse_str(run_id).is_err() {
        return false;
    }
    let Some(outcomes) = record["outcomes"].as_array() else {
        return false;
    };
    if outcomes.len() != CONCURRENT_START_COUNT {
        return false;
    }
    outcomes
        .iter()
        .all(|outcome| match record["boundary"].as_str() {
            Some("initial_hash") => {
                outcome.as_object().map(|object| object.len()) == Some(2)
                    && outcome["status"] == "rejected"
                    && outcome["code"] == "BUSY"
            }
            Some("worker_preflight") => {
                outcome.as_object().map(|object| object.len()) == Some(4)
                    && outcome["status"] == "fulfilled"
                    && outcome["state"] == "accepted"
                    && outcome["runId"] == run_id
                    && outcome["jobId"]
                        .as_str()
                        .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
            }
            _ => false,
        })
}
