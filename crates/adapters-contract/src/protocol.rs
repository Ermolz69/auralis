use ports::error::PortError;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::time::{Duration, timeout};

pub const MAX_FRAME_BYTES: usize = 65_536;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerEvent {
    pub protocol_version: u8,
    pub request_id: String,
    pub project_id: String,
    pub run_id: String,
    pub attempt_id: String,
    pub sequence: u64,
    pub kind: String,
    pub payload: Value,
}

pub async fn send_request(
    writer: &mut (impl AsyncWrite + Unpin),
    ids: (&str, &str, &str, &str),
    operation: &str,
    payload: Value,
) -> Result<(), PortError> {
    let (request_id, project_id, run_id, attempt_id) = ids;
    let mut encoded = serde_json::to_vec(&json!({
        "protocol_version": 1,
        "request_id": request_id,
        "project_id": project_id,
        "run_id": run_id,
        "attempt_id": attempt_id,
        "operation": operation,
        "payload": payload,
    }))
    .map_err(|error| invalid(&error.to_string()))?;
    encoded.push(b'\n');
    if encoded.len() > MAX_FRAME_BYTES {
        return Err(invalid("worker request exceeds frame budget"));
    }
    writer.write_all(&encoded).await.map_err(io_error)?;
    writer.flush().await.map_err(io_error)
}

pub async fn read_event(
    reader: &mut (impl AsyncRead + Unpin),
    expected: (&str, &str, &str, &str, u64),
    deadline: Duration,
) -> Result<WorkerEvent, PortError> {
    let frame = timeout(deadline, async {
        let mut frame = Vec::new();
        loop {
            let mut byte = [0_u8; 1];
            let count = reader.read(&mut byte).await.map_err(io_error)?;
            if count == 0 {
                return Err(invalid("worker EOF before a complete event"));
            }
            frame.push(byte[0]);
            if frame.len() > MAX_FRAME_BYTES {
                return Err(invalid("worker response exceeds frame budget"));
            }
            if byte[0] == b'\n' {
                return Ok(frame);
            }
        }
    })
    .await
    .map_err(|_| invalid("worker response deadline exceeded"))??;
    let event: WorkerEvent = serde_json::from_slice(&frame)
        .map_err(|error| invalid(&format!("malformed worker event: {error}")))?;
    let (request_id, project_id, run_id, attempt_id, sequence) = expected;
    if event.protocol_version != 1
        || event.request_id != request_id
        || event.project_id != project_id
        || event.run_id != run_id
        || event.attempt_id != attempt_id
        || event.sequence != sequence
    {
        return Err(invalid("stale or mismatched worker event"));
    }
    Ok(event)
}

fn invalid(message: &str) -> PortError {
    PortError::ExternalToolFailed {
        tool: "CONTRACT worker".into(),
        message: message.into(),
    }
}

fn io_error(error: std::io::Error) -> PortError {
    PortError::Io {
        message: error.to_string(),
    }
}
