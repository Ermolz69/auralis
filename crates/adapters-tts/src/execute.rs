use async_trait::async_trait;
use domain::project::ProjectId;
use ports::cancellation::CancellationToken;
use ports::error::PortError;
use process_wrap::tokio::ChildWrapper;
use serde::Deserialize;
use serde_json::json;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;

use crate::client::TtsWorkerClient;
use crate::owned_process;
use crate::protocol::{WorkerIdentity, WorkerRequest};
use crate::result_verification::verify_result_files_cancellable;
use crate::verification_control::VerificationGuard;

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
        let cancellation = cancellation.child_token();
        let _verification_guard = VerificationGuard(cancellation.clone());
        request.validate(self)?;
        if cancellation.is_cancelled() {
            return Ok(TtsExecutionOutcome::Cancelled);
        }
        self.verify_profile_integrity(&request.profile_path).await?;
        if cancellation.is_cancelled() {
            return Ok(TtsExecutionOutcome::Cancelled);
        }
        let admission =
            tokio::task::spawn_blocking(common::model_admission::ModelAdmissionLease::acquire)
                .await
                .map_err(|e| unexpected(&e.to_string()))?
                .map_err(|e| PortError::Busy {
                    message: e.to_string(),
                })?;
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
            cancellation.clone(),
            self.runtime().execution_budget,
        )
        .await;
        match result {
            Ok(TtsExecutionOutcome::Completed(completed)) => {
                let completed = tokio::task::spawn_blocking(move || {
                    verify_result_files_cancellable(&request, &completed, &cancellation)?;
                    Ok::<_, PortError>(completed)
                })
                .await
                .map_err(|error| unexpected(&error.to_string()))??;
                Ok(TtsExecutionOutcome::Completed(completed))
            }
            Ok(outcome) => Ok(outcome),
            Err(error) => {
                if let Err(cleanup) = terminate(&mut child).await {
                    std::mem::forget(admission);
                    return Err(cleanup);
                }
                Err(error)
            }
        }
    }
}

async fn terminate(child: &mut Box<dyn ChildWrapper>) -> Result<(), PortError> {
    let _ = child.start_kill();
    timeout(EXIT_DEADLINE, child.wait())
        .await
        .map_err(|_| PortError::CleanupFailed {
            resource: "TTS worker".into(),
            message: "TTS worker termination could not be confirmed; model admission retained"
                .into(),
        })?
        .map_err(|e| PortError::CleanupFailed {
            resource: "TTS worker".into(),
            message: format!(
                "TTS worker exit could not be confirmed: {e}; model admission retained"
            ),
        })?;
    Ok(())
}

mod validation;
mod worker;
pub(crate) use validation::confined;
use validation::{invalid_event, io_error, unexpected};
use worker::run_worker;

#[cfg(all(test, windows))]
mod tests;

#[cfg(test)]
mod cleanup_tests;
