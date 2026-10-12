use async_trait::async_trait;
use ports::{
    error::PortError,
    marker::{MarkerPort, MarkerPreparation},
    speech_workflow::SpeechSourceSnapshot,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::io::AsyncReadExt;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkerRuntimeConfig {
    pub version: u8,
    pub node_executable: PathBuf,
    pub package_root: PathBuf,
    pub release_sha256: String,
}

pub struct MarkerClient {
    pub config: MarkerRuntimeConfig,
}

#[async_trait]
impl MarkerPort for MarkerClient {
    async fn prepare(
        &self,
        original: SpeechSourceSnapshot,
    ) -> Result<MarkerPreparation, PortError> {
        if self.config.version != 1
            || !self.config.node_executable.is_absolute()
            || !self.config.node_executable.is_file()
            || !self.config.package_root.is_absolute()
        {
            return Err(invalid("Настройка маркировщика недействительна"));
        }
        let release_bytes = tokio::fs::read(self.config.package_root.join("release.json"))
            .await
            .map_err(|e| invalid(e.to_string()))?;
        if format!("{:x}", Sha256::digest(&release_bytes)) != self.config.release_sha256 {
            return Err(invalid(
                "Версия маркировщика изменилась; повторите настройку",
            ));
        }
        let release: serde_json::Value =
            serde_json::from_slice(&release_bytes).map_err(|e| invalid(e.to_string()))?;
        let files = release
            .get("files")
            .or_else(|| release.get("fileHashes"))
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| invalid("Некорректный реестр маркировщика"))?;
        for (name, expected) in files {
            if name.contains("..")
                || name.starts_with('/')
                || name.contains('\\')
                || name.contains(':')
            {
                return Err(invalid("Небезопасный путь в реестре маркировщика"));
            }
            let bytes = tokio::fs::read(self.config.package_root.join(name))
                .await
                .map_err(|e| invalid(e.to_string()))?;
            if expected.as_str() != Some(format!("{:x}", Sha256::digest(bytes)).as_str()) {
                return Err(invalid(format!("Файл маркировщика изменён: {name}")));
            }
        }
        let lease = tokio::task::spawn_blocking(
            common::model_admission::ModelAdmissionLease::acquire_model_free,
        )
        .await
        .map_err(|e| invalid(e.to_string()))?
        .map_err(|e| PortError::Busy {
            message: e.to_string(),
        })?;
        let directory = tempfile::tempdir().map_err(|e| invalid(e.to_string()))?;
        let input = directory.path().join("source.txt");
        let output = directory.path().join("prepared.txt");
        let report = directory.path().join("report.json");
        let document = !original.cues.is_empty();
        let source = if document {
            serde_json::json!({"blocks":original.cues.iter().map(|cue| serde_json::json!({"id":cue.id,"source":cue.text})).collect::<Vec<_>>()}).to_string()
        } else {
            original.text.clone()
        };
        tokio::fs::write(&input, source)
            .await
            .map_err(|e| invalid(e.to_string()))?;
        let mut command = tokio::process::Command::new(&self.config.node_executable);
        command
            .arg(self.config.package_root.join("cli.mjs"))
            .arg("--input")
            .arg(&input)
            .arg("--output")
            .arg(&output)
            .arg("--report")
            .arg(&report)
            .arg("--format")
            .arg(if document { "document" } else { "text" })
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped());
        let mut child = crate::owned_process::spawn(command).map_err(|e| invalid(e.to_string()))?;
        let process_id = child.id();
        let result = tokio::time::timeout(std::time::Duration::from_secs(60), async {
            let stderr = child
                .stderr()
                .take()
                .ok_or_else(|| invalid("Marker stderr unavailable"))?;
            let mut errors = Vec::new();
            stderr
                .take(8193)
                .read_to_end(&mut errors)
                .await
                .map_err(|e| invalid(e.to_string()))?;
            let status = child.wait().await.map_err(|e| invalid(e.to_string()))?;
            if !status.success() {
                return Err(invalid(format!(
                    "Маркировщик: {}",
                    String::from_utf8_lossy(&errors)
                )));
            }
            let bytes = tokio::fs::read(&report)
                .await
                .map_err(|e| invalid(e.to_string()))?;
            if bytes.len() > 16_777_216 {
                return Err(invalid("Отчёт маркировщика слишком большой"));
            }
            let report: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|e| invalid(e.to_string()))?;
            if report["modelRequests"] != 0 || report["version"] != "1.0.0" {
                return Err(invalid(
                    "Неожиданный модельный вызов или версия маркировщика",
                ));
            }
            build_preparation(original, &report["result"], document)
        })
        .await;
        if !matches!(result, Ok(Ok(_))) {
            let _ = Box::into_pin(child.kill()).await;
            child
                .wait()
                .await
                .map_err(|e| invalid(format!("Не удалось завершить маркировщик: {e}")))?;
            super::marker_lock::release_owned(&self.config.package_root, process_id).await?;
        }
        drop(lease);
        match result {
            Ok(result) => result,
            Err(_) => Err(invalid("Маркировщик не завершился вовремя")),
        }
    }
}

pub(super) fn build_preparation(
    original: SpeechSourceSnapshot,
    result: &serde_json::Value,
    document: bool,
) -> Result<MarkerPreparation, PortError> {
    let mut prepared = original.clone();
    let mut changed = 0usize;
    let mut unresolved = 0usize;
    let blocks = if document {
        result["blocks"]
            .as_array()
            .ok_or_else(|| invalid("Marker block coverage is missing"))?
            .clone()
    } else {
        vec![result.clone()]
    };
    if document && blocks.len() != original.cues.len() {
        return Err(invalid("Marker cue coverage changed"));
    }
    for (q, block) in blocks.iter().enumerate() {
        let text = block["text"]
            .as_str()
            .ok_or_else(|| invalid("Marker text is missing"))?;
        if document {
            if block["id"].as_str() != Some(original.cues[q].id.as_str()) {
                return Err(invalid("Marker cue order changed"));
            }
            prepared.cues[q].text = text.into();
        } else {
            prepared.text = text.into();
        }
        changed += block["replacements"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|value| {
                block["targets"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|target| target["id"] == value["id"] && target["raw"] != value["text"])
            })
            .count();
        unresolved += block["unresolved"].as_array().map_or(0, Vec::len);
    }
    if document {
        prepared.text = prepared
            .cues
            .iter()
            .map(|cue| cue.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
    }
    prepared.sha256 = format!(
        "{:x}",
        Sha256::digest(if document {
            serde_json::to_vec(&prepared.cues).map_err(|e| invalid(e.to_string()))?
        } else {
            prepared.text.as_bytes().to_vec()
        })
    );
    Ok(MarkerPreparation {
        version: "1.0.0".into(),
        original,
        prepared,
        changed_targets: changed as u32,
        unresolved_targets: unresolved as u32,
    })
}

fn invalid(message: impl Into<String>) -> PortError {
    PortError::InvalidSource {
        message: message.into(),
    }
}
