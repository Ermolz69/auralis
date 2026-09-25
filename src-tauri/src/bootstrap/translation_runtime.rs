use crate::bootstrap::paths::AppPaths;
use adapters_translate::{ManagedLlamaRuntime, ManagedRuntimeConfig};
use ports::error::PortError;
use ports::translation_model_runtime::TranslationModelRuntimePort;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

const MAX_CONFIG_BYTES: u64 = 16 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TranslationRuntimeSettings {
    executable: PathBuf,
    model_file: PathBuf,
    context_tokens: u32,
    gpu_layers: u32,
    startup_timeout_seconds: u64,
    readiness_interval_milliseconds: u64,
}

pub fn load(paths: &AppPaths) -> Result<Option<Arc<dyn TranslationModelRuntimePort>>, PortError> {
    let path = paths.translation_runtime_config();
    let bytes = match std::fs::metadata(&path) {
        Ok(metadata) if metadata.len() <= MAX_CONFIG_BYTES => {
            std::fs::read(&path).map_err(io_error)?
        }
        Ok(_) => {
            return Err(PortError::InvalidSource {
                message: "translation runtime configuration exceeds size limit".into(),
            });
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io_error(error)),
    };
    let settings: TranslationRuntimeSettings =
        serde_json::from_slice(&bytes).map_err(|error| PortError::InvalidSource {
            message: format!("translation runtime configuration is invalid: {error}"),
        })?;
    let config = ManagedRuntimeConfig {
        executable: settings.executable,
        model_file: settings.model_file,
        context_tokens: settings.context_tokens,
        gpu_layers: settings.gpu_layers,
        startup_timeout: Duration::from_secs(settings.startup_timeout_seconds),
        readiness_interval: Duration::from_millis(settings.readiness_interval_milliseconds),
    };
    let runtime = ManagedLlamaRuntime::new(config)?;
    Ok(Some(Arc::new(runtime)))
}

fn io_error(error: std::io::Error) -> PortError {
    PortError::Io {
        message: error.to_string(),
    }
}
