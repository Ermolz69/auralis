use super::config::ManagedRuntimeConfig;
use super::lease::ManagedLlamaLease;
use super::owned_process;
use super::preparation::RuntimePreparation;
use async_trait::async_trait;
use auralis_translation_llamacpp::{
    LlamaCppProvider, ModelProfile, PreparationControl, verify_server_with_control,
};
use ports::cancellation::CancellationToken;
use ports::error::PortError;
use ports::translation_links::TranslationRunIntent;
use ports::translation_model_runtime::{TranslationModelLease, TranslationModelRuntimePort};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;
use tokio::process::Command;
use tokio::sync::Semaphore;

const LOOPBACK_HOST: &str = "127.0.0.1";
const SERVER_TOOL: &str = "managed llama.cpp server";

mod model;

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

    async fn wait_until_ready(
        &self,
        lease: &mut ManagedLlamaLease,
        profile: ModelProfile,
        model_path: PathBuf,
        cancellation: CancellationToken,
    ) -> Result<(), PortError> {
        let deadline = Instant::now() + self.config.startup_timeout;
        let control = RuntimePreparation::new(cancellation.clone(), Some(deadline));
        loop {
            control.check().map_err(|cause| control.error(cause))?;
            if lease.exited().map_err(io_error)? {
                return Err(tool_error("server exited before model preflight"));
            }
            let url = lease.server_url().to_owned();
            let profile_to_probe = profile.clone();
            let probe_control = control.clone();
            let reported = tokio::task::spawn_blocking(move || {
                let provider = LlamaCppProvider::new(&url, profile_to_probe)
                    .map_err(|error| error.to_string())?;
                Ok::<_, String>(provider.probe_server_with_control(&probe_control))
            })
            .await
            .map_err(|error| unexpected(&error.to_string()))?
            .map_err(|error| tool_error(&error))?;
            control.check().map_err(|cause| control.error(cause))?;
            if let Ok(report) = reported {
                let reported_path = std::fs::canonicalize(&report.model_path).map_err(io_error)?;
                if reported_path != model_path {
                    return Err(conflict("server loaded a different model path"));
                }
                let url = lease.server_url().to_owned();
                let profile_to_verify = profile.clone();
                let verify_control = control.clone();
                tokio::task::spawn_blocking(move || {
                    let provider = LlamaCppProvider::new(&url, profile_to_verify.clone())?;
                    verify_server_with_control(&provider, &profile_to_verify, &verify_control)
                })
                .await
                .map_err(|error| unexpected(&error.to_string()))?
                .map_err(|error| control.error(error))?;
                control.check().map_err(|cause| control.error(cause))?;
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
            tokio::select! {
                _ = cancellation.cancelled() => return Err(PortError::Cancelled),
                _ = tokio::time::sleep(self.config.readiness_interval) => {}
            }
        }
    }
}

#[async_trait]
impl TranslationModelRuntimePort for ManagedLlamaRuntime {
    async fn acquire_with_control(
        &self,
        intent: &TranslationRunIntent,
        cancellation: CancellationToken,
    ) -> Result<Box<dyn TranslationModelLease>, PortError> {
        let cancel_on_drop = cancellation.clone().drop_guard();
        let control = RuntimePreparation::new(cancellation.clone(), None);
        control.check().map_err(|cause| control.error(cause))?;
        let permit = self
            .slot
            .clone()
            .try_acquire_owned()
            .map_err(|_| PortError::Busy {
                message: "a managed translation model is already active".into(),
            })?;
        self.validate_model_input(intent, &control)?;
        let admission =
            tokio::task::spawn_blocking(common::model_admission::ModelAdmissionLease::acquire)
                .await
                .map_err(|e| unexpected(&e.to_string()))?
                .map_err(|e| PortError::Busy {
                    message: e.to_string(),
                })?;
        let (profile, model_path) = self.verify_model(intent, &control).await?;
        control.check().map_err(|cause| control.error(cause))?;
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
            .arg("--cache-ram")
            .arg(self.config.ram_cache_mib.to_string())
            .arg("--parallel")
            .arg("1")
            .arg("--jinja")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let child = owned_process::spawn(command).map_err(io_error)?;
        let url = format!("http://{LOOPBACK_HOST}:{port}/");
        let mut lease = ManagedLlamaLease::new(url, child, permit, admission);
        let result = self
            .wait_until_ready(&mut lease, profile, model_path, cancellation)
            .await;
        if let Err(error) = result {
            lease
                .stop()
                .await
                .map_err(|cause| PortError::CleanupFailed {
                    resource: SERVER_TOOL.into(),
                    message: cause.to_string(),
                })?;
            return Err(error);
        }
        cancel_on_drop.disarm();
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
