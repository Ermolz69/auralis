use crate::TtsWorkerClient;
use ports::error::PortError;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Component, Path},
};

#[derive(Deserialize)]
struct Inventory {
    version: u8,
    package_files: Vec<FileHash>,
    model_files: Vec<ModelHash>,
    dependencies: Vec<FileHash>,
}
#[derive(Deserialize)]
struct FileHash {
    path: String,
    sha256: String,
}
#[derive(Deserialize)]
struct ModelHash {
    engine: String,
    path: String,
    sha256: String,
}

impl TtsWorkerClient {
    pub(crate) async fn verify_profile_integrity(&self, profile: &Path) -> Result<(), PortError> {
        let bytes = tokio::fs::read(profile).await.map_err(io_error)?;
        let value: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|e| invalid(&format!("TTS profile is invalid: {e}")))?;
        let engine = value["engine"]
            .as_str()
            .filter(|engine| matches!(*engine, "voxcpm2" | "espeech"))
            .ok_or_else(|| invalid("TTS profile has an unsupported engine"))?;
        self.verify_integrity(Some(engine.to_owned())).await
    }

    pub(crate) async fn verify_integrity(&self, engine: Option<String>) -> Result<(), PortError> {
        let config = self.runtime().clone();
        tokio::task::spawn_blocking(move || {
            let receipt = config.package_root.join("integrity.json");
            verify_file(&receipt, &config.integrity_sha256)?;
            let inventory: Inventory =
                serde_json::from_slice(&std::fs::read(receipt).map_err(io_error)?)
                    .map_err(|e| invalid(&format!("TTS runtime inventory is invalid: {e}")))?;
            if inventory.version != 1
                || inventory.package_files.is_empty()
                || inventory.dependencies.is_empty()
            {
                return Err(invalid("TTS runtime inventory is incomplete"));
            }
            for file in inventory.package_files {
                verify_file(&relative(&config.package_root, &file.path)?, &file.sha256)?;
            }
            for file in inventory.dependencies {
                verify_file(Path::new(&file.path), &file.sha256)?;
            }
            if let Some(engine) = engine {
                let files: Vec<_> = inventory
                    .model_files
                    .into_iter()
                    .filter(|f| f.engine == engine)
                    .collect();
                if files.is_empty() {
                    return Err(invalid("Pinned model hashes are missing for this engine"));
                }
                for file in files {
                    verify_file(&relative(&config.model_cache, &file.path)?, &file.sha256)?;
                }
            }
            Ok(())
        })
        .await
        .map_err(|e| invalid(&e.to_string()))?
    }
}

fn relative(root: &Path, value: &str) -> Result<std::path::PathBuf, PortError> {
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(invalid("TTS inventory contains an unsafe path"));
    }
    Ok(root.join(path))
}
fn verify_file(path: &Path, expected: &str) -> Result<(), PortError> {
    let mut stream = std::fs::File::open(path).map_err(|e| {
        invalid(&format!(
            "TTS runtime file is unavailable: {}: {e}",
            path.display()
        ))
    })?;
    let mut digest = Sha256::new();
    let mut buffer = vec![0; 1024 * 1024];
    loop {
        let count = stream.read(&mut buffer).map_err(io_error)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    if format!("{:x}", digest.finalize()) != expected {
        return Err(invalid(&format!(
            "TTS runtime or model hash changed: {}",
            path.display()
        )));
    }
    Ok(())
}
fn io_error(e: std::io::Error) -> PortError {
    PortError::Io {
        message: e.to_string(),
    }
}
fn invalid(message: &str) -> PortError {
    PortError::InvalidSource {
        message: message.into(),
    }
}
