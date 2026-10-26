use domain::project::ProjectId;
use ports::error::PortError;
use serde_json::Value;
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use uuid::Uuid;

use crate::owned_process;
use crate::protocol::{
    MAX_FRAME_BYTES, WorkerEvent, WorkerIdentity, WorkerProtocol, WorkerRequest,
};

const MAX_DIAGNOSTIC_BYTES: usize = 8192;
const PREFLIGHT_DEADLINE: Duration = Duration::from_secs(45);
const PINNED_VOXCPM_REVISION: &str = "32279effe8c19989596f05d353d1447f51d9e915";

#[derive(Clone, Debug)]
pub struct TtsRuntimeConfig {
    pub execution_budget: ports::speech_run::SpeechExecutionBudget,
    pub python_executable: PathBuf,
    pub package_root: PathBuf,
    pub voice_store: PathBuf,
    pub profile_root: PathBuf,
    pub model_cache: PathBuf,
    pub managed_root: PathBuf,
    pub voxcpm_revision: String,
    pub integrity_sha256: String,
}

#[derive(Clone, Copy)]
pub(crate) enum PreflightOperation {
    Capabilities,
    Catalog,
    Compose,
    Validate,
}

impl PreflightOperation {
    fn as_str(self) -> &'static str {
        match self {
            Self::Capabilities => "capabilities",
            Self::Catalog => "catalog",
            Self::Compose => "compose",
            Self::Validate => "validate",
        }
    }

    fn response_kind(self) -> &'static str {
        match self {
            Self::Capabilities => "capabilities",
            Self::Catalog => "catalog",
            Self::Compose => "composed",
            Self::Validate => "validated",
        }
    }
}

#[derive(Clone)]
pub struct TtsWorkerClient {
    runtime: TtsRuntimeConfig,
}

impl TtsWorkerClient {
    pub fn new(runtime: TtsRuntimeConfig) -> Result<Self, PortError> {
        if !runtime.python_executable.is_absolute()
            || !runtime.package_root.is_absolute()
            || !runtime.package_root.join("src/tts/worker.py").is_file()
            || !runtime.voice_store.is_absolute()
            || !runtime.profile_root.is_absolute()
            || !runtime.model_cache.is_absolute()
            || !runtime.managed_root.is_absolute()
            || runtime.voxcpm_revision != PINNED_VOXCPM_REVISION
            || runtime.integrity_sha256.len() != 64
        {
            return Err(PortError::InvalidSource {
                message: "Invalid pinned TTS runtime configuration".into(),
            });
        }
        Ok(Self { runtime })
    }

    pub(crate) fn worker_command(&self) -> Command {
        let mut command = Command::new(&self.runtime.python_executable);
        command
            .arg("-I")
            .arg("-c")
            .arg("import sys; sys.path.insert(0, sys.argv[1]); from src.tts.worker import main; raise SystemExit(main())")
            .arg(&self.runtime.package_root)
            .env_remove("PYTHONPATH")
            .env_remove("PYTHONHOME")
            .env("PYTHONNOUSERSITE", "1")
            .env("TTS_REQUIRE_PRELOADED_MODELS", "1")
            .env("TTS_VOXCPM_REVISION", &self.runtime.voxcpm_revision)
            .env("PUZA_TTS_PROFILE_DIR", &self.runtime.profile_root)
            .env("HF_HOME", &self.runtime.model_cache)
            .env("HF_HUB_CACHE", self.runtime.model_cache.join("hub"))
            .env("HF_HUB_OFFLINE", "1")
            .current_dir(&self.runtime.package_root)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        command
    }

    pub(crate) fn runtime(&self) -> &TtsRuntimeConfig {
        &self.runtime
    }

    pub(crate) async fn preflight(
        &self,
        project: &ProjectId,
        run_id: &str,
        attempt_id: &str,
        operation: PreflightOperation,
        payload: Value,
    ) -> Result<WorkerEvent, PortError> {
        if !portable_id(run_id) || !portable_id(attempt_id) {
            return Err(PortError::InvalidSource {
                message: "Invalid TTS run or attempt ID".into(),
            });
        }
        if !self.runtime.python_executable.is_file() {
            return Err(PortError::Unsupported {
                message: "TTS runtime is not installed".into(),
            });
        }
        let identity = WorkerIdentity {
            request_id: Uuid::new_v4().to_string(),
            project_id: project.to_string(),
            run_id: run_id.into(),
            attempt_id: attempt_id.into(),
        };
        let request = WorkerRequest::encode(&identity, operation.as_str(), payload)?;
        let mut child =
            owned_process::spawn(self.worker_command()).map_err(|error| PortError::Io {
                message: error.to_string(),
            })?;
        let mut stdin = child.stdin().take().ok_or_else(|| PortError::Unexpected {
            message: "TTS worker stdin unavailable".into(),
        })?;
        stdin
            .write_all(&request)
            .await
            .map_err(|error| PortError::Io {
                message: error.to_string(),
            })?;
        stdin.shutdown().await.map_err(|error| PortError::Io {
            message: error.to_string(),
        })?;
        drop(stdin);
        let stdout = child.stdout().take().ok_or_else(|| PortError::Unexpected {
            message: "TTS worker stdout unavailable".into(),
        })?;
        let stderr = child.stderr().take().ok_or_else(|| PortError::Unexpected {
            message: "TTS worker stderr unavailable".into(),
        })?;
        let outcome = tokio::time::timeout(PREFLIGHT_DEADLINE, async {
            let read_stdout = async {
                let mut frame = Vec::new();
                stdout
                    .take(MAX_FRAME_BYTES as u64 + 1)
                    .read_to_end(&mut frame)
                    .await
                    .map(|_| frame)
            };
            let read_stderr = async {
                let mut diagnostic = Vec::new();
                stderr
                    .take(MAX_DIAGNOSTIC_BYTES as u64 + 1)
                    .read_to_end(&mut diagnostic)
                    .await
                    .map(|_| diagnostic)
            };
            tokio::try_join!(read_stdout, read_stderr, child.wait())
        })
        .await;
        let (frame, diagnostic, status) = match outcome {
            Ok(Ok(value)) => value,
            Ok(Err(error)) => {
                return Err(PortError::Io {
                    message: error.to_string(),
                });
            }
            Err(_) => {
                let _ = Box::into_pin(child.kill()).await;
                let _ = child.wait().await;
                return Err(PortError::ExternalToolFailed {
                    tool: "puza-tts-worker".into(),
                    message: "Preflight deadline exceeded".into(),
                });
            }
        };
        let mut protocol = WorkerProtocol::default();
        let event = protocol.parse(&frame, &identity)?;
        if event.event_kind == "error" {
            let code = event
                .payload
                .get("code")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let message = event
                .payload
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("TTS worker rejected the request");
            return Err(PortError::InvalidSource {
                message: format!("{code}: {message}"),
            });
        }
        if !status.success()
            || event.event_kind != operation.response_kind()
            || diagnostic.len() > MAX_DIAGNOSTIC_BYTES
        {
            return Err(PortError::ExternalToolFailed {
                tool: "puza-tts-worker".into(),
                message: "Incomplete or unexpected TTS worker response".into(),
            });
        }
        Ok(event)
    }
}

fn portable_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}
