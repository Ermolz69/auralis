use crate::config::WorkerConfig;
use crate::protocol::{WorkerEvent, read_event, send_request};
use async_trait::async_trait;
use common::owned_process;
use domain::speech::{SpeechId, SpeechPlan, SynthesisResult, VoiceSnapshot};
use ports::error::PortError;
use ports::speech_contract::{SpeechContractIssue, SpeechContractPort, SpeechContractReport};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::process::Stdio;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capabilities {
    package_version: String,
    protocol_versions: Vec<u8>,
    speech_plan_versions: Vec<u8>,
    segment_result_versions: Vec<u8>,
    synthesis_result_versions: Vec<u8>,
    operations: Vec<String>,
    synthesis_available: bool,
    schema_sha256: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct WorkerReport {
    version: u8,
    scope: String,
    valid: bool,
    errors: Vec<WorkerIssue>,
}

#[derive(Debug, Deserialize)]
struct WorkerIssue {
    code: String,
    explanation: String,
}

pub struct ContractWorkerAdapter {
    config: WorkerConfig,
}

impl ContractWorkerAdapter {
    pub fn new(config: WorkerConfig) -> Result<Self, PortError> {
        config.validate()?;
        Ok(Self { config })
    }

    async fn exchange(
        &self,
        project_id: &SpeechId,
        scope: &str,
        payload: Value,
    ) -> Result<SpeechContractReport, PortError> {
        let executable =
            std::fs::canonicalize(&self.config.executable).map_err(|error| PortError::Io {
                message: error.to_string(),
            })?;
        let cwd = std::fs::canonicalize(&self.config.working_directory).map_err(|error| {
            PortError::Io {
                message: error.to_string(),
            }
        })?;
        let mut command = Command::new(executable);
        command
            .args(&self.config.arguments)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = owned_process::spawn(command).map_err(|error| PortError::Io {
            message: error.to_string(),
        })?;
        let mut stdin = match child.take_stdin() {
            Some(handle) => handle,
            None => {
                child.stop().await.map_err(cleanup_error)?;
                return Err(worker_error("worker stdin is unavailable"));
            }
        };
        let mut stdout = match child.take_stdout() {
            Some(handle) => handle,
            None => {
                child.stop().await.map_err(cleanup_error)?;
                return Err(worker_error("worker stdout is unavailable"));
            }
        };
        let stderr = match child.take_stderr() {
            Some(handle) => handle,
            None => {
                child.stop().await.map_err(cleanup_error)?;
                return Err(worker_error("worker stderr is unavailable"));
            }
        };
        let diagnostics = tokio::spawn(drain_stderr(stderr));
        let exchange = self
            .perform(project_id, scope, payload, &mut stdin, &mut stdout)
            .await;
        drop(stdin);
        if exchange.is_err() {
            child.stop().await.map_err(cleanup_error)?;
            diagnostics
                .await
                .map_err(|error| worker_error(&error.to_string()))??;
            return exchange;
        }
        let exited = timeout(self.config.exit_timeout, child.wait()).await;
        match exited {
            Ok(Ok(status)) if status.success() => {}
            Ok(Ok(_)) => {
                return Err(worker_error(
                    "worker exited unsuccessfully after validation",
                ));
            }
            Ok(Err(error)) => return Err(cleanup_error(error)),
            Err(_) => {
                child.stop().await.map_err(cleanup_error)?;
                return Err(worker_error("worker did not exit after validation"));
            }
        }
        diagnostics
            .await
            .map_err(|error| worker_error(&error.to_string()))??;
        exchange
    }

    async fn perform(
        &self,
        project_id: &SpeechId,
        scope: &str,
        payload: Value,
        stdin: &mut (impl AsyncWrite + Unpin),
        stdout: &mut (impl AsyncRead + Unpin),
    ) -> Result<SpeechContractReport, PortError> {
        let project = project_id.as_str();
        let run = "contract-check";
        let attempt = "contract-attempt";
        send_request(
            stdin,
            ("capability-01", project, run, attempt),
            "capabilities",
            json!({
                "required_protocol_version": self.config.pin.protocol_version,
                "required_speech_plan_version": self.config.pin.speech_plan_version,
            }),
        )
        .await?;
        let capability = read_event(
            stdout,
            ("capability-01", project, run, attempt, 1),
            self.config.capability_timeout,
        )
        .await?;
        self.check_capabilities(capability)?;
        send_request(
            stdin,
            ("validate-01", project, run, attempt),
            "validate",
            payload,
        )
        .await?;
        let event = read_event(
            stdout,
            ("validate-01", project, run, attempt, 2),
            self.config.validation_timeout,
        )
        .await?;
        if event.kind != "validation" {
            return Err(worker_error(
                "unknown or unsuccessful validation completion",
            ));
        }
        let report = event
            .payload
            .get("report")
            .ok_or_else(|| worker_error("worker validation report is missing"))?;
        let report: WorkerReport = serde_json::from_value(report.clone())
            .map_err(|error| worker_error(&format!("invalid worker report: {error}")))?;
        if report.version != 1
            || report.scope != scope
            || report.valid != report.errors.is_empty()
            || report
                .errors
                .iter()
                .any(|issue| issue.code.is_empty() || issue.explanation.is_empty())
        {
            return Err(worker_error(
                "worker report identity or validity is inconsistent",
            ));
        }
        Ok(SpeechContractReport {
            scope: report.scope,
            valid: report.valid,
            errors: report
                .errors
                .into_iter()
                .map(|error| SpeechContractIssue {
                    code: error.code,
                    explanation: error.explanation,
                })
                .collect(),
        })
    }

    fn check_capabilities(&self, event: WorkerEvent) -> Result<(), PortError> {
        if event.kind != "capabilities" {
            return Err(worker_error("worker capability negotiation failed"));
        }
        let capabilities: Capabilities = serde_json::from_value(event.payload)
            .map_err(|error| worker_error(&format!("invalid worker capabilities: {error}")))?;
        if capabilities.package_version != self.config.pin.package_version
            || capabilities.protocol_versions != [1]
            || capabilities.speech_plan_versions != [1]
            || capabilities.segment_result_versions != [1]
            || capabilities.synthesis_result_versions != [1]
            || capabilities.operations != ["capabilities", "validate"]
            || capabilities.synthesis_available
            || capabilities.schema_sha256 != self.config.pin.schema_sha256
        {
            return Err(worker_error(
                "installed CONTRACT package or schema identity differs from pin",
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl SpeechContractPort for ContractWorkerAdapter {
    async fn validate_plan(&self, plan: &SpeechPlan) -> Result<SpeechContractReport, PortError> {
        self.exchange(
            &plan.project_id,
            "speech_plan",
            json!({"kind": "speech_plan", "plan": plan}),
        )
        .await
    }

    async fn validate_voice(
        &self,
        snapshot: &VoiceSnapshot,
    ) -> Result<SpeechContractReport, PortError> {
        let project =
            SpeechId::parse("voice-catalog").map_err(|message| PortError::InvalidSource {
                message: message.into(),
            })?;
        self.exchange(
            &project,
            "voice_snapshot",
            json!({"kind": "voice_snapshot", "snapshot": snapshot}),
        )
        .await
    }

    async fn validate_result(
        &self,
        plan: &SpeechPlan,
        result: &SynthesisResult,
    ) -> Result<SpeechContractReport, PortError> {
        self.exchange(
            &plan.project_id,
            "synthesis_result",
            json!({"kind": "synthesis_result", "plan": plan, "result": result}),
        )
        .await
    }
}

async fn drain_stderr(mut reader: impl AsyncRead + Unpin) -> Result<Vec<u8>, PortError> {
    let mut buffer = [0_u8; 4096];
    let mut retained = Vec::new();
    loop {
        let count = reader
            .read(&mut buffer)
            .await
            .map_err(|error| PortError::Io {
                message: error.to_string(),
            })?;
        if count == 0 {
            return Ok(retained);
        }
        let room = 65_536_usize.saturating_sub(retained.len());
        retained.extend_from_slice(&buffer[..count.min(room)]);
    }
}

fn worker_error(message: &str) -> PortError {
    PortError::ExternalToolFailed {
        tool: "CONTRACT worker".into(),
        message: message.into(),
    }
}

fn cleanup_error(error: std::io::Error) -> PortError {
    PortError::CleanupFailed {
        resource: "CONTRACT worker process tree".into(),
        message: error.to_string(),
    }
}
