use async_trait::async_trait;
use ports::{error::PortError, media_project::MediaContractValidator};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::io::AsyncReadExt;

/// Independent CONTRACT installation: upgrading the media format must not change old TTS run receipts.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaContractRuntime {
    pub version: u32,
    pub python_executable: PathBuf,
    pub package_root: PathBuf,
    pub package_version: String,
    pub files: Vec<ContractFile>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractFile {
    pub path: String,
    pub sha256: String,
}

impl MediaContractRuntime {
    pub fn load(path: &Path) -> Result<Self, PortError> {
        let runtime: Self =
            serde_json::from_slice(&std::fs::read(path).map_err(error)?).map_err(error)?;
        if runtime.version != 1
            || runtime.package_version != "0.3.0"
            || !runtime.python_executable.is_absolute()
            || !runtime.package_root.is_absolute()
            || runtime.files.is_empty()
        {
            return Err(error(
                "Нужен установленный медиаконтракт CONTRACT 0.3.0: task setup:project:local",
            ));
        }
        runtime.verify()?;
        Ok(runtime)
    }
    fn verify(&self) -> Result<(), PortError> {
        for file in &self.files {
            if file.path.contains(['\\', ':'])
                || file
                    .path
                    .split('/')
                    .any(|p| p.is_empty() || p == "." || p == "..")
            {
                return Err(error("Небезопасный путь пакета CONTRACT"));
            }
            let bytes = std::fs::read(self.package_root.join(&file.path)).map_err(error)?;
            if format!("{:x}", Sha256::digest(bytes)) != file.sha256 {
                return Err(error(format!("Изменён пакет CONTRACT: {}", file.path)));
            }
        }
        Ok(())
    }
}
#[async_trait]
impl MediaContractValidator for MediaContractRuntime {
    async fn validate(&self, root: &Path) -> Result<(), PortError> {
        self.verify()?;
        let mut command = tokio::process::Command::new(&self.python_executable);
        command.arg("-I").arg("-X").arg("utf8").arg("-c")
            .arg("import sys,importlib.metadata; from pathlib import Path; sys.path.insert(0,sys.argv[1]); assert importlib.metadata.version('voxscript-contract')=='0.3.0'; from voxscript_contract.media_project import validate_media_project; validate_media_project(Path(sys.argv[2]))")
            .arg(&self.package_root).arg(root).env_remove("PYTHONPATH").env_remove("PYTHONHOME")
            .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::piped());
        let mut child = crate::owned_process::spawn(command).map_err(error)?;
        let outcome = tokio::time::timeout(std::time::Duration::from_secs(120), async {
            let mut stderr = Vec::new();
            child
                .stderr()
                .take()
                .ok_or_else(|| error("Нет stderr CONTRACT"))?
                .take(16384)
                .read_to_end(&mut stderr)
                .await
                .map_err(error)?;
            let status = child.wait().await.map_err(error)?;
            if !status.success() {
                return Err(error(String::from_utf8_lossy(&stderr)));
            }
            Ok(())
        })
        .await;
        match outcome {
            Ok(Ok(())) => Ok(()),
            other => {
                let _ = Box::into_pin(child.kill()).await;
                child.wait().await.map_err(error)?;
                match other {
                    Ok(Err(e)) => Err(e),
                    _ => Err(error("Истекло время проверки CONTRACT")),
                }
            }
        }
    }
}
pub struct UnavailableMediaContract(pub String);
#[async_trait]
impl MediaContractValidator for UnavailableMediaContract {
    async fn validate(&self, _root: &Path) -> Result<(), PortError> {
        Err(error(&self.0))
    }
}
fn error(value: impl std::fmt::Display) -> PortError {
    PortError::InvalidSource {
        message: format!("CONTRACT: {value}"),
    }
}
