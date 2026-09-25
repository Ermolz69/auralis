use super::config::ManagedRuntimeConfig;
use super::lease::ManagedLlamaLease;
use super::owned_process;
use async_trait::async_trait;
use auralis_translation::SourceHash;
use auralis_translation_llamacpp::{LlamaCppProvider, ModelProfile, hash_file, verify_server};
use ports::error::PortError;
use ports::translation_links::TranslationRunIntent;
use ports::translation_model_runtime::{TranslationModelLease, TranslationModelRuntimePort};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Semaphore;
use tokio::time::Instant;

const LOOPBACK_HOST: &str = "127.0.0.1";
const SERVER_TOOL: &str = "managed llama.cpp server";

pub struct ManagedLlamaRuntime {
    config: ManagedRuntimeConfig,
    slot: Arc<Semaphore>,
}

impl ManagedLlamaRuntime {
    pub fn new(config: ManagedRuntimeConfig) -> Result<Self, PortError> {
        config.validate()?;
        Ok(Self {
            config,
            slot: Arc::new(Semaphore::new(1)),
        })
    }

    async fn verify_model(
        &self,
        intent: &TranslationRunIntent,
    ) -> Result<(ModelProfile, PathBuf), PortError> {
        let bytes = intent.profile_manifest_json.as_bytes();
        if SourceHash::digest(bytes).to_string() != intent.profile_sha256 {
            return Err(conflict(
                "frozen model profile digest differs from its bytes",
            ));
        }
        let profile =
            ModelProfile::from_json(bytes).map_err(|error| invalid(&error.to_string()))?;
        let expected_bytes = profile
            .model_file_bytes
            .ok_or_else(|| PortError::Unsupported {
                message: "managed execution requires a checked model profile".into(),
            })?;
        let minimum_context = profile
            .min_context_tokens
            .ok_or_else(|| invalid("checked model profile has no minimum context"))?;
        if self.config.context_tokens < minimum_context {
            return Err(invalid(
                "configured context is smaller than checked profile",
            ));
        }
        self.config.validate_files()?;
        let path = std::fs::canonicalize(&self.config.model_file).map_err(io_error)?;
        if std::fs::metadata(&path).map_err(io_error)?.len() != expected_bytes {
            return Err(conflict(
                "installed model size differs from checked profile",
            ));
        }
        let expected_hash = SourceHash::parse_hex(&profile.model_file_sha256)
            .ok_or_else(|| invalid("checked model digest is invalid"))?;
        let path_for_hash = path.clone();
        let (actual_hash, actual_bytes) =
            tokio::task::spawn_blocking(move || hash_file(&path_for_hash))
                .await
                .map_err(|error| unexpected(&error.to_string()))?
                .map_err(|error| tool_error(&error.to_string()))?;
        if actual_bytes != expected_bytes || actual_hash != expected_hash {
            return Err(conflict(
                "installed model digest differs from checked profile",
            ));
        }
        Ok((profile, path))
    }

    async fn wait_until_ready(
        &self,
        lease: &mut ManagedLlamaLease,
        profile: ModelProfile,
        model_path: PathBuf,
    ) -> Result<(), PortError> {
        let deadline = Instant::now() + self.config.startup_timeout;
        loop {
            if lease.exited().map_err(io_error)? {
                return Err(tool_error("server exited before model preflight"));
            }
            let url = lease.server_url().to_owned();
            let profile_to_probe = profile.clone();
            let reported = tokio::time::timeout_at(
                deadline,
                tokio::task::spawn_blocking(move || {
                    let provider = LlamaCppProvider::new(&url, profile_to_probe)
                        .map_err(|error| error.to_string())?;
                    Ok::<_, String>(provider.probe_server())
                }),
            )
            .await
            .map_err(|_| tool_error("server did not become ready before startup timeout"))?
            .map_err(|error| unexpected(&error.to_string()))?
            .map_err(|error| tool_error(&error))?;
            if let Ok(report) = reported {
                let reported_path = std::fs::canonicalize(&report.model_path).map_err(io_error)?;
                if reported_path != model_path {
                    return Err(conflict("server loaded a different model path"));
                }
                let url = lease.server_url().to_owned();
                let profile_to_verify = profile.clone();
                tokio::time::timeout_at(
                    deadline,
                    tokio::task::spawn_blocking(move || {
                        let provider = LlamaCppProvider::new(&url, profile_to_verify.clone())?;
                        verify_server(&provider, &profile_to_verify)
                    }),
                )
                .await
                .map_err(|_| tool_error("model preflight exceeded startup timeout"))?
                .map_err(|error| unexpected(&error.to_string()))?
                .map_err(|error| tool_error(&error.to_string()))?;
                if lease.exited().map_err(io_error)? {
                    return Err(tool_error("server exited during model preflight"));
                }
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(tool_error(
                    "server did not become ready before startup timeout",
                ));
            }
            tokio::time::sleep(self.config.readiness_interval).await;
        }
    }
}

#[async_trait]
impl TranslationModelRuntimePort for ManagedLlamaRuntime {
    async fn acquire(
        &self,
        intent: &TranslationRunIntent,
    ) -> Result<Box<dyn TranslationModelLease>, PortError> {
        let permit = self
            .slot
            .clone()
            .try_acquire_owned()
            .map_err(|_| PortError::Busy {
                message: "a managed translation model is already active".into(),
            })?;
        let (profile, model_path) = self.verify_model(intent).await?;
        let listener = TcpListener::bind((LOOPBACK_HOST, 0)).map_err(io_error)?;
        let port = listener.local_addr().map_err(io_error)?.port();
        drop(listener);
        let executable = std::fs::canonicalize(&self.config.executable).map_err(io_error)?;
        let mut command = Command::new(executable);
        command
            .arg("--model")
            .arg(&model_path)
            .arg("--alias")
            .arg(&profile.model_alias)
            .arg("--host")
            .arg(LOOPBACK_HOST)
            .arg("--port")
            .arg(port.to_string())
            .arg("-c")
            .arg(self.config.context_tokens.to_string())
            .arg("-ngl")
            .arg(self.config.gpu_layers.to_string())
            .arg("--jinja")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let child = owned_process::spawn(command).map_err(io_error)?;
        let url = format!("http://{LOOPBACK_HOST}:{port}/");
        let mut lease = ManagedLlamaLease::new(url, child, permit);
        self.wait_until_ready(&mut lease, profile, model_path)
            .await?;
        Ok(Box::new(lease))
    }
}

fn invalid(message: &str) -> PortError {
    PortError::InvalidSource {
        message: message.into(),
    }
}

fn conflict(message: &str) -> PortError {
    PortError::Conflict {
        resource: "managed model runtime".into(),
        message: message.into(),
    }
}

fn io_error(error: std::io::Error) -> PortError {
    PortError::Io {
        message: error.to_string(),
    }
}

fn tool_error(message: &str) -> PortError {
    PortError::ExternalToolFailed {
        tool: SERVER_TOOL.into(),
        message: message.into(),
    }
}

fn unexpected(message: &str) -> PortError {
    PortError::Unexpected {
        message: message.into(),
    }
}
