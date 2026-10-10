use ports::error::PortError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: u8 = 1;
pub const MAX_FRAME_BYTES: usize = 1_048_576;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerIdentity {
    pub request_id: String,
    pub project_id: String,
    pub run_id: String,
    pub attempt_id: String,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerRequest<'a> {
    protocol_version: u8,
    request_id: &'a str,
    project_id: &'a str,
    run_id: &'a str,
    attempt_id: &'a str,
    operation: &'a str,
    payload: Value,
}

impl WorkerRequest<'_> {
    pub fn encode<'a>(
        identity: &'a WorkerIdentity,
        operation: &'a str,
        payload: Value,
    ) -> Result<Vec<u8>, PortError> {
        let request = WorkerRequest {
            protocol_version: PROTOCOL_VERSION,
            request_id: &identity.request_id,
            project_id: &identity.project_id,
            run_id: &identity.run_id,
            attempt_id: &identity.attempt_id,
            operation,
            payload,
        };
        let mut frame = serde_json::to_vec(&request).map_err(|error| PortError::InvalidSource {
            message: error.to_string(),
        })?;
        frame.push(b'\n');
        if frame.len() > MAX_FRAME_BYTES {
            return Err(PortError::InvalidSource {
                message: "TTS worker request exceeds frame limit".into(),
            });
        }
        Ok(frame)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerEvent {
    protocol_version: u8,
    request_id: String,
    project_id: String,
    run_id: String,
    attempt_id: String,
    event_sequence: u64,
    pub event_kind: String,
    pub payload: Value,
}

#[derive(Default)]
pub struct WorkerProtocol {
    next_sequence: u64,
}

impl WorkerProtocol {
    pub fn parse(
        &mut self,
        frame: &[u8],
        expected: &WorkerIdentity,
    ) -> Result<WorkerEvent, PortError> {
        self.parse_one_of(frame, &[expected])
    }

    pub fn parse_one_of(
        &mut self,
        frame: &[u8],
        expected: &[&WorkerIdentity],
    ) -> Result<WorkerEvent, PortError> {
        let primary = expected.first().ok_or_else(|| PortError::Unexpected {
            message: "TTS protocol has no expected identity".into(),
        })?;
        if frame.is_empty()
            || frame.len() > MAX_FRAME_BYTES
            || !frame.ends_with(b"\n")
            || frame[..frame.len() - 1].contains(&b'\n')
        {
            return Err(PortError::InvalidStoredData {
                entity_type: "tts_worker".into(),
                entity_id: primary.request_id.clone(),
                field: "frame".into(),
                message: "Malformed or oversized worker frame".into(),
            });
        }
        let event: WorkerEvent =
            serde_json::from_slice(&frame[..frame.len() - 1]).map_err(|error| {
                PortError::InvalidStoredData {
                    entity_type: "tts_worker".into(),
                    entity_id: primary.request_id.clone(),
                    field: "event".into(),
                    message: error.to_string(),
                }
            })?;
        if event.protocol_version != PROTOCOL_VERSION
            || !expected.iter().any(|identity| {
                event.request_id == identity.request_id
                    && event.project_id == identity.project_id
                    && event.run_id == identity.run_id
                    && event.attempt_id == identity.attempt_id
            })
            || event.event_sequence != self.next_sequence + 1
        {
            return Err(PortError::Conflict {
                resource: "tts_worker".into(),
                message: "Stale, reordered or mismatched worker event".into(),
            });
        }
        self.next_sequence += 1;
        Ok(event)
    }
}
