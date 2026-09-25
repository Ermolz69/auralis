use ports::error::PortError;
use std::path::PathBuf;
use std::time::Duration;

const MAX_STARTUP_TIMEOUT: Duration = Duration::from_secs(600);
const MIN_READINESS_INTERVAL: Duration = Duration::from_millis(50);
const MAX_READINESS_INTERVAL: Duration = Duration::from_secs(5);

pub struct ManagedRuntimeConfig {
    pub executable: PathBuf,
    pub model_file: PathBuf,
    pub context_tokens: u32,
    pub gpu_layers: u32,
    pub startup_timeout: Duration,
    pub readiness_interval: Duration,
}

impl ManagedRuntimeConfig {
    pub fn validate(&self) -> Result<(), PortError> {
        if self.context_tokens == 0
            || self.startup_timeout.is_zero()
            || self.startup_timeout > MAX_STARTUP_TIMEOUT
            || self.readiness_interval < MIN_READINESS_INTERVAL
            || self.readiness_interval > MAX_READINESS_INTERVAL
            || self.readiness_interval > self.startup_timeout
        {
            return Err(PortError::InvalidSource {
                message: "managed model runtime limits are invalid".into(),
            });
        }
        if !self.executable.is_absolute() || !self.model_file.is_absolute() {
            return Err(PortError::InvalidSource {
                message: "managed runtime executable and model paths must be absolute".into(),
            });
        }
        Ok(())
    }

    pub fn validate_files(&self) -> Result<(), PortError> {
        for (name, path) in [
            ("llama.cpp executable", &self.executable),
            ("model file", &self.model_file),
        ] {
            if !path.is_file() {
                return Err(PortError::NotFound {
                    resource: format!("{name}: {}", path.display()),
                });
            }
        }
        Ok(())
    }
}
