use ports::error::PortError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractPin {
    pub package_name: String,
    pub package_version: String,
    pub protocol_version: u8,
    pub speech_plan_version: u8,
    pub schema_sha256: BTreeMap<String, String>,
}

impl ContractPin {
    pub fn bundled() -> Result<Self, PortError> {
        let pin: Self =
            serde_json::from_str(include_str!("../contract-pin-v1.json")).map_err(|error| {
                PortError::InvalidSource {
                    message: error.to_string(),
                }
            })?;
        pin.validate()?;
        Ok(pin)
    }

    pub fn validate(&self) -> Result<(), PortError> {
        if self.package_name != "voxscript-contract"
            || self.package_version != "0.2.0"
            || self.protocol_version != 1
            || self.speech_plan_version != 1
            || self.schema_sha256.len() != 4
            || [
                "speech-plan-foundation-v1.schema.json",
                "voice-snapshot-v1.schema.json",
                "segment-result-v1.schema.json",
                "synthesis-result-v1.schema.json",
            ]
            .iter()
            .any(|name| !self.schema_sha256.contains_key(*name))
            || self.schema_sha256.values().any(|hash| {
                hash.len() != 64
                    || !hash
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
        {
            return Err(PortError::InvalidSource {
                message: "invalid CONTRACT package or schema pin".into(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct WorkerConfig {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub working_directory: PathBuf,
    pub pin: ContractPin,
    pub capability_timeout: Duration,
    pub validation_timeout: Duration,
    pub exit_timeout: Duration,
}

impl WorkerConfig {
    pub fn validate(&self) -> Result<(), PortError> {
        self.pin.validate()?;
        if !self.executable.is_absolute()
            || !self.working_directory.is_absolute()
            || self.arguments.is_empty()
            || self
                .arguments
                .iter()
                .any(|arg| arg.is_empty() || arg.len() > 4096)
            || self.capability_timeout.is_zero()
            || self.validation_timeout.is_zero()
            || self.exit_timeout.is_zero()
            || self.capability_timeout > Duration::from_secs(5)
            || self.validation_timeout > Duration::from_secs(10)
            || self.exit_timeout > Duration::from_secs(5)
        {
            return Err(PortError::InvalidSource {
                message: "invalid validator worker configuration".into(),
            });
        }
        Ok(())
    }
}
