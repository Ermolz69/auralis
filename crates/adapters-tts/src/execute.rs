use async_trait::async_trait;
use domain::project::ProjectId;
use ports::cancellation::CancellationToken;
use ports::error::PortError;
use process_wrap::tokio::ChildWrapper;
use serde::Deserialize;
use serde_json::json;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::time::{Instant, timeout};
use uuid::Uuid;

use crate::client::TtsWorkerClient;
use crate::framed::WorkerFrameReader;
use crate::owned_process;
use crate::protocol::{WorkerIdentity, WorkerProtocol, WorkerRequest};
use crate::result_verification::verify_result_files;

const EXECUTION_DEADLINE: Duration = Duration::from_secs(45 * 60);
const CANCELLATION_DEADLINE: Duration = Duration::from_secs(20);
const EXIT_DEADLINE: Duration = Duration::from_secs(5);
const MAX_DIAGNOSTIC_BYTES: usize = 8192;

#[derive(Clone, Debug)]
pub struct TtsExecutionRequest {
    pub project_id: ProjectId,
    pub run_id: String,
    pub attempt_id: String,
    pub plan_digest: String,
    pub expected_utterances: usize,
    pub expected_chunks: usize,
    pub plan_path: PathBuf,
    pub profile_path: PathBuf,
    pub out_file: PathBuf,
    pub manifest_path: PathBuf,
    pub cache_dir: PathBuf,
    pub checkpoint_root: PathBuf,
    pub rebuild: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedCheckpoint {
    pub sequence: u64,
    pub run_id: String,
    pub attempt_id: String,
    pub plan_digest: String,
    pub utterance_id: String,
    pub cue_id: Option<String>,
    pub chunk_id: String,
    pub cache_key: String,
    pub audio_sha256: String,
    pub duration_ms: u64,
    pub status: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TtsCompleted {
    pub output: PathBuf,
    pub output_sha256: String,
    pub manifest: PathBuf,
    pub manifest_sha256: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: u64,
    pub duration_ms: u64,
    pub utterance_count: usize,
    pub chunk_count: usize,
}

#[derive(Clone, Debug)]
pub enum TtsExecutionOutcome {
    Completed(TtsCompleted),
    Cancelled,
}

#[async_trait]
pub trait TtsExecutionObserver: Send + Sync {
    async fn saved_checkpoint(&self, checkpoint: &SavedCheckpoint) -> Result<(), PortError>;
}

impl TtsWorkerClient {
    pub async fn execute(
        &self,
        request: TtsExecutionRequest,
        observer: &dyn TtsExecutionObserver,
        cancellation: CancellationToken,
    ) -> Result<TtsExecutionOutcome, PortError> {
        request.validate(self)?;
        if cancellation.is_cancelled() {
            return Ok(TtsExecutionOutcome::Cancelled);
        }
        let identity = WorkerIdentity {
            request_id: Uuid::new_v4().to_string(),
            project_id: request.project_id.to_string(),
            run_id: request.run_id.clone(),
            attempt_id: request.attempt_id.clone(),
        };
        let payload = json!({
            "plan_path": request.plan_path,
            "voice_store": self.runtime().voice_store,
            "profile_path": request.profile_path,
            "out_file": request.out_file,
            "manifest_path": request.manifest_path,
            "cache_dir": request.cache_dir,
            "checkpoint_root": request.checkpoint_root,
            "rebuild": request.rebuild,
        });
        let command = WorkerRequest::encode(&identity, "execute", payload)?;
        let mut child = owned_process::spawn(self.worker_command()).map_err(io_error)?;
        let result = run_worker(
            &mut child,
            &command,
            &identity,
            &request,
            observer,
            cancellation,
        )
        .await;
        match result {
            Ok(TtsExecutionOutcome::Completed(completed)) => {
                let completed = tokio::task::spawn_blocking(move || {
                    verify_result_files(&request, &completed)?;
                    Ok::<_, PortError>(completed)
                })
                .await
                .map_err(|error| unexpected(&error.to_string()))??;
                Ok(TtsExecutionOutcome::Completed(completed))
            }
            Ok(outcome) => Ok(outcome),
            Err(error) => {
                terminate(&mut child).await?;
                Err(error)
            }
        }
    }
}

async fn run_worker(
    child: &mut Box<dyn ChildWrapper>,
    command: &[u8],
    identity: &WorkerIdentity,
    request: &TtsExecutionRequest,
    observer: &dyn TtsExecutionObserver,
    cancellation: CancellationToken,
) -> Result<TtsExecutionOutcome, PortError> {
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
    stdin.write_all(command).await.map_err(io_error)?;
    let mut framed = WorkerFrameReader::new(stdout);
    let mut protocol = WorkerProtocol::default();
    let mut cancel_identity = None;
    let mut deadline = Instant::now() + EXECUTION_DEADLINE;
    let mut started = false;
    let mut checkpoints = 0usize;
    let terminal = loop {
        let frame = tokio::select! {
            biased;
            _ = cancellation.cancelled(), if cancel_identity.is_none() => {
                let cancel = WorkerIdentity { request_id: Uuid::new_v4().to_string(), ..identity.clone() };
                let frame = WorkerRequest::encode(&cancel, "cancel", json!({}))?;
                stdin.write_all(&frame).await.map_err(io_error)?;
                cancel_identity = Some(cancel);
                deadline = Instant::now() + CANCELLATION_DEADLINE;
                continue;
            }
            result = tokio::time::timeout_at(deadline, framed.next_frame()) => {
                result.map_err(|_| PortError::ExternalToolFailed {
                    tool: "puza-tts-worker".into(),
                    message: "Worker or cancellation deadline exceeded".into(),
                })?.map_err(io_error)?
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
                observer.saved_checkpoint(&checkpoint).await?;
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
    let status = match timeout(EXIT_DEADLINE, child.wait()).await {
        Ok(Ok(status)) => status,
        Ok(Err(error)) => return Err(io_error(error)),
        Err(_) => {
            return Err(unexpected(
                "TTS worker did not exit after its terminal event",
            ));
        }
    };
    let _diagnostic = diagnostic_task
        .await
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

async fn terminate(child: &mut Box<dyn ChildWrapper>) -> Result<(), PortError> {
    let _ = Box::into_pin(child.kill()).await;
    timeout(EXIT_DEADLINE, child.wait())
        .await
        .map_err(|_| unexpected("TTS worker termination could not be confirmed"))?
        .map_err(io_error)?;
    Ok(())
}

impl TtsExecutionRequest {
    fn validate(&self, client: &TtsWorkerClient) -> Result<(), PortError> {
        if !portable_id(&self.run_id)
            || !portable_id(&self.attempt_id)
            || !hex_sha256(&self.plan_digest)
            || self.expected_utterances == 0
            || self.expected_chunks < self.expected_utterances
        {
            return Err(invalid_event(
                "Invalid TTS execution identity or expected coverage",
            ));
        }
        let runtime = client.runtime();
        if !runtime.python_executable.is_file()
            || !runtime.voice_store.is_dir()
            || !runtime.managed_root.is_dir()
            || !runtime.profile_root.is_dir()
            || !self.plan_path.is_file()
            || !self.profile_path.is_file()
        {
            return Err(PortError::Unsupported {
                message: "TTS runtime or inputs are unavailable".into(),
            });
        }
        for path in [
            &self.plan_path,
            &self.out_file,
            &self.manifest_path,
            &self.cache_dir,
            &self.checkpoint_root,
        ] {
            confined(&runtime.managed_root, path)?;
        }
        confined(&runtime.profile_root, &self.profile_path)?;
        Ok(())
    }
}

impl SavedCheckpoint {
    fn validate(
        &self,
        request: &TtsExecutionRequest,
        expected_sequence: usize,
    ) -> Result<(), PortError> {
        if self.sequence != expected_sequence as u64
            || self.run_id != request.run_id
            || self.attempt_id != request.attempt_id
            || self.plan_digest != request.plan_digest
            || !portable_id(&self.utterance_id)
            || !portable_id(&self.chunk_id)
            || !hex_sha256(&self.audio_sha256)
            || !hex_sha256(&self.cache_key)
            || self.duration_ms == 0
            || !matches!(
                self.status.as_str(),
                "generated" | "cached" | "reused_in_job"
            )
            || expected_sequence > request.expected_chunks
        {
            return Err(invalid_event("Invalid or stale saved TTS checkpoint"));
        }
        Ok(())
    }
}

impl TtsCompleted {
    fn validate(&self, request: &TtsExecutionRequest, checkpoints: usize) -> Result<(), PortError> {
        if self.output != request.out_file
            || self.manifest != request.manifest_path
            || !hex_sha256(&self.output_sha256)
            || !hex_sha256(&self.manifest_sha256)
            || self.sample_rate == 0
            || self.channels == 0
            || self.frames == 0
            || self.duration_ms == 0
            || self.utterance_count != request.expected_utterances
            || self.chunk_count != request.expected_chunks
            || checkpoints != request.expected_chunks
        {
            return Err(invalid_event("Incomplete or mismatched TTS completion"));
        }
        Ok(())
    }
}

pub(crate) fn confined(root: &Path, path: &Path) -> Result<(), PortError> {
    if !path.is_absolute()
        || !path.starts_with(root)
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(invalid_event("TTS path escapes its managed root"));
    }
    let mut current = root.to_path_buf();
    if current.is_symlink() {
        return Err(invalid_event("TTS managed root is a link"));
    }
    for component in path
        .strip_prefix(root)
        .map_err(|_| invalid_event("TTS path escapes its managed root"))?
        .components()
    {
        current.push(component);
        if current.is_symlink() {
            return Err(invalid_event("TTS path contains a link"));
        }
    }
    Ok(())
}

fn portable_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn io_error(error: std::io::Error) -> PortError {
    PortError::Io {
        message: error.to_string(),
    }
}

fn unexpected(message: &str) -> PortError {
    PortError::ExternalToolFailed {
        tool: "puza-tts-worker".into(),
        message: message.into(),
    }
}

fn invalid_event(message: impl Into<String>) -> PortError {
    PortError::InvalidStoredData {
        entity_type: "tts_worker".into(),
        entity_id: "event".into(),
        field: "event".into(),
        message: message.into(),
    }
}
