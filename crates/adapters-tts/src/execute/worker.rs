use super::{
    CANCELLATION_DEADLINE, EXIT_DEADLINE, MAX_DIAGNOSTIC_BYTES, SavedCheckpoint, TtsCompleted,
    TtsExecutionObserver, TtsExecutionOutcome, TtsExecutionRequest, invalid_event, io_error,
    unexpected,
};
use crate::{
    framed::WorkerFrameReader,
    protocol::{WorkerIdentity, WorkerProtocol, WorkerRequest},
};
use ports::{PortError, cancellation::CancellationToken, speech_run::SpeechExecutionBudget};
use process_wrap::tokio::ChildWrapper;
use serde_json::json;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    time::{Instant, timeout_at},
};
use uuid::Uuid;

fn deadline_error(cancelled: bool, budget: SpeechExecutionBudget, phase: &str) -> PortError {
    PortError::ExternalToolFailed {
        tool: "puza-tts-worker".into(),
        message: if cancelled {
            format!(
                "TTS cancellation deadline exceeded ({} seconds) while {phase}",
                CANCELLATION_DEADLINE.as_secs()
            )
        } else {
            format!(
                "TTS execution deadline exceeded ({} seconds) while {phase}",
                budget.seconds()
            )
        },
    }
}

pub(super) async fn run_worker(
    child: &mut Box<dyn ChildWrapper>,
    command: &[u8],
    identity: &WorkerIdentity,
    request: &TtsExecutionRequest,
    observer: &dyn TtsExecutionObserver,
    cancellation: CancellationToken,
    budget: SpeechExecutionBudget,
) -> Result<TtsExecutionOutcome, PortError> {
    let mut deadline = Instant::now() + budget.duration();
    let mut stdin = child
        .stdin()
        .take()
        .ok_or_else(|| unexpected("TTS worker stdin unavailable"))?;
    let stdout = child
        .stdout()
        .take()
        .ok_or_else(|| unexpected("TTS worker stdout unavailable"))?;
    let stderr = child
        .stderr()
        .take()
        .ok_or_else(|| unexpected("TTS worker stderr unavailable"))?;
    let diagnostic_task = tokio::spawn(drain_diagnostic(stderr));
    dispatch_request(&mut stdin, command, deadline, &cancellation, budget).await?;
    let mut framed = WorkerFrameReader::new(stdout);
    let mut protocol = WorkerProtocol::default();
    let mut cancel_identity = None;
    let mut started = false;
    let mut checkpoints = 0usize;
    let terminal = loop {
        let frame = tokio::select! {
            biased;
            _ = cancellation.cancelled(), if cancel_identity.is_none() => {
                let cancel = WorkerIdentity { request_id: Uuid::new_v4().to_string(), ..identity.clone() };
                let frame = WorkerRequest::encode(&cancel, "cancel", json!({}))?;
                deadline = Instant::now() + CANCELLATION_DEADLINE;
                timeout_at(deadline, stdin.write_all(&frame)).await
                    .map_err(|_| deadline_error(true, budget, "dispatching cancellation"))?.map_err(io_error)?;
                cancel_identity = Some(cancel);
                continue;
            }
            result = timeout_at(deadline, framed.next_frame()) => {
                result.map_err(|_| deadline_error(cancel_identity.is_some(), budget, "reading worker event"))?.map_err(io_error)?
            }
        };
        let frame = frame.ok_or_else(|| unexpected("TTS worker exited before a terminal event"))?;
        let expected = cancel_identity
            .as_ref()
            .map_or_else(|| vec![identity], |cancel| vec![identity, cancel]);
        let event = protocol.parse_one_of(&frame, &expected)?;
        match event.event_kind.as_str() {
            "started" if !started && event.payload == json!({}) => started = true,
            "saved_checkpoint" if started => {
                let checkpoint: SavedCheckpoint = serde_json::from_value(event.payload)
                    .map_err(|error| invalid_event(error.to_string()))?;
                checkpoints += 1;
                checkpoint.validate(request, checkpoints)?;
                if cancel_identity.is_none() {
                    let persisted = tokio::select! {
                        biased;
                        _ = cancellation.cancelled() => false,
                        result = timeout_at(deadline, observer.saved_checkpoint(&checkpoint)) => {
                            result.map_err(|_| deadline_error(false, budget, "persisting checkpoint"))??;
                            true
                        }
                    };
                    if !persisted {
                        continue;
                    }
                }
            }
            "cancel_requested" if cancel_identity.is_some() && event.payload == json!({}) => {}
            "completed" if started && cancel_identity.is_none() => {
                let completed: TtsCompleted = serde_json::from_value(event.payload)
                    .map_err(|error| invalid_event(error.to_string()))?;
                completed.validate(request, checkpoints)?;
                break TtsExecutionOutcome::Completed(completed);
            }
            "cancelled" if cancel_identity.is_some() => break TtsExecutionOutcome::Cancelled,
            "error" => {
                let code = event
                    .payload
                    .get("code")
                    .and_then(|value| value.as_str())
                    .unwrap_or("unknown");
                let message = event
                    .payload
                    .get("message")
                    .and_then(|value| value.as_str())
                    .unwrap_or("Worker failed");
                return Err(PortError::ExternalToolFailed {
                    tool: "puza-tts-worker".into(),
                    message: format!("{code}: {message}"),
                });
            }
            _ => return Err(invalid_event("Unexpected TTS worker event or event order")),
        }
    };
    drop(stdin);
    let exit_deadline = Instant::now() + EXIT_DEADLINE;
    let status = match timeout_at(exit_deadline, child.wait()).await {
        Ok(Ok(status)) => status,
        Ok(Err(error)) => return Err(io_error(error)),
        Err(_) => {
            return Err(unexpected(
                "TTS worker did not exit after its terminal event",
            ));
        }
    };
    let _diagnostic = timeout_at(exit_deadline, diagnostic_task)
        .await
        .map_err(|_| unexpected("TTS worker diagnostics did not close before the exit deadline"))?
        .map_err(|error| unexpected(&error.to_string()))?
        .map_err(io_error)?;
    if (matches!(terminal, TtsExecutionOutcome::Completed(_)) && !status.success())
        || (matches!(terminal, TtsExecutionOutcome::Cancelled) && status.code() != Some(130))
    {
        return Err(unexpected(
            "TTS worker exit or diagnostic limit differed from its terminal event",
        ));
    }
    Ok(terminal)
}

pub(super) async fn dispatch_request(
    stdin: &mut (impl AsyncWrite + Unpin),
    command: &[u8],
    deadline: Instant,
    cancellation: &CancellationToken,
    budget: SpeechExecutionBudget,
) -> Result<(), PortError> {
    tokio::select! {
        biased;
        _ = cancellation.cancelled() => Err(PortError::Cancelled),
        result = timeout_at(deadline, stdin.write_all(command)) => {
            result.map_err(|_| deadline_error(false, budget, "dispatching request"))?.map_err(io_error)
        }
    }
}

async fn drain_diagnostic(mut stderr: impl AsyncRead + Unpin) -> std::io::Result<(Vec<u8>, bool)> {
    let mut diagnostic = Vec::new();
    let mut overflow = false;
    let mut buffer = [0u8; 4096];
    loop {
        let read = stderr.read(&mut buffer).await?;
        if read == 0 {
            return Ok((diagnostic, overflow));
        }
        let available = MAX_DIAGNOSTIC_BYTES.saturating_sub(diagnostic.len());
        diagnostic.extend_from_slice(&buffer[..read.min(available)]);
        overflow |= read > available;
    }
}
