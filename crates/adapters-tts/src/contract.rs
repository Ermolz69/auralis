use async_trait::async_trait;
use domain::project::ProjectId;
use ports::{
    error::PortError,
    puza_contract::{PuzaContractPort, PuzaContractSnapshot, PuzaVoiceBinding},
};
use serde_json::json;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

use crate::{TtsWorkerClient, owned_process};

#[async_trait]
impl PuzaContractPort for TtsWorkerClient {
    async fn prepare_project(
        &self,
        project: &ProjectId,
        source_text: Option<String>,
        bindings: Vec<PuzaVoiceBinding>,
    ) -> Result<PuzaContractSnapshot, PortError> {
        self.verify_integrity(None).await?;
        let request_id = Uuid::new_v4().to_string();
        let request = json!({"request_id":request_id,"project_id":project.to_string(),
            "source_text":source_text,"bindings":bindings,
            "max_bindings":ports::speech_run::MAX_SPEECH_UTTERANCES + 1})
        .to_string();
        if request.len() > 2_097_152 {
            return Err(invalid("CONTRACT request exceeds its limit"));
        }
        let config = self.runtime();
        let mut command = tokio::process::Command::new(&config.python_executable);
        command
            .arg("-I")
            .arg("-X")
            .arg("utf8")
            .arg("-c")
            .arg(include_str!("contract_worker.py"))
            .arg(&config.package_root)
            .arg(&config.voice_store)
            .env_remove("PYTHONPATH")
            .env_remove("PYTHONHOME")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        let mut child = owned_process::spawn(command).map_err(|e| invalid(e.to_string()))?;
        let result = tokio::time::timeout(std::time::Duration::from_secs(45), async {
            let mut stdin = child
                .stdin()
                .take()
                .ok_or_else(|| invalid("CONTRACT stdin is missing"))?;
            stdin
                .write_all(request.as_bytes())
                .await
                .map_err(|e| invalid(e.to_string()))?;
            stdin.shutdown().await.map_err(|e| invalid(e.to_string()))?;
            drop(stdin);
            let stdout = child
                .stdout()
                .take()
                .ok_or_else(|| invalid("CONTRACT stdout is missing"))?;
            let stderr = child
                .stderr()
                .take()
                .ok_or_else(|| invalid("CONTRACT stderr is missing"))?;
            let read = async {
                let mut bytes = Vec::new();
                stdout
                    .take(4_194_305)
                    .read_to_end(&mut bytes)
                    .await
                    .map_err(|e| invalid(e.to_string()))?;
                Ok::<_, PortError>(bytes)
            };
            let diagnostics = async {
                let mut bytes = Vec::new();
                stderr
                    .take(8193)
                    .read_to_end(&mut bytes)
                    .await
                    .map_err(|e| invalid(e.to_string()))?;
                Ok::<_, PortError>(bytes)
            };
            let (bytes, errors) = tokio::try_join!(read, diagnostics)?;
            let status = child.wait().await.map_err(|e| invalid(e.to_string()))?;
            if !status.success() || bytes.len() > 4_194_304 {
                return Err(invalid(format!(
                    "CONTRACT: {}",
                    String::from_utf8_lossy(&errors)
                )));
            }
            let response: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|e| invalid(e.to_string()))?;
            if response["request_id"] != request_id || response["project_id"] != project.to_string()
            {
                return Err(invalid("CONTRACT response identity differs"));
            }
            let snapshot: PuzaContractSnapshot = serde_json::from_value(response["result"].clone())
                .map_err(|e| invalid(e.to_string()))?;
            if snapshot.contract_version != "0.2.0" || snapshot.sha256.len() != 64 {
                return Err(invalid("Invalid CONTRACT project receipt"));
            }
            let mut value = serde_json::to_value(&snapshot).map_err(|e| invalid(e.to_string()))?;
            value
                .as_object_mut()
                .ok_or_else(|| invalid("Invalid CONTRACT receipt"))?
                .remove("sha256");
            let canonical = serde_json::to_vec(&value).map_err(|e| invalid(e.to_string()))?;
            if format!("{:x}", Sha256::digest(canonical)) != snapshot.sha256 {
                return Err(invalid("CONTRACT project receipt hash differs"));
            }
            tracing::info!(event_name = "contract_snapshot_verified", contract_version = %snapshot.contract_version, project_id = %project, "CONTRACT project metadata verified");
            Ok(snapshot)
        })
        .await;
        match result {
            Ok(Ok(snapshot)) => Ok(snapshot),
            other => {
                let _ = Box::into_pin(child.kill()).await;
                child
                    .wait()
                    .await
                    .map_err(|error| invalid(format!("CONTRACT worker cleanup failed: {error}")))?;
                match other {
                    Ok(Err(error)) => Err(error),
                    _ => Err(invalid("CONTRACT worker timed out")),
                }
            }
        }
    }
}

fn invalid(message: impl Into<String>) -> PortError {
    PortError::InvalidSource {
        message: message.into(),
    }
}
