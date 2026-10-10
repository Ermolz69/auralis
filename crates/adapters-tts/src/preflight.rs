use domain::project::ProjectId;
use ports::error::PortError;
use serde::Deserialize;
use serde_json::json;
use std::path::{Path, PathBuf};

use crate::client::{PreflightOperation, TtsWorkerClient};
use crate::execute::confined;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TtsProfile {
    pub profile_id: String,
    pub profile_sha256: String,
    pub engine_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TtsCapabilities {
    pub speech_plan_versions: Vec<u8>,
    pub protocol_versions: Vec<u8>,
    pub supported_engines: Vec<String>,
    pub operations: Vec<String>,
    pub installed_profiles: Vec<TtsProfile>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TtsCatalogReference {
    pub reference_id: String,
    pub engine_id: String,
    pub emotion_id: String,
    pub transcript: String,
    pub audio_sha256: String,
    pub qualification: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TtsCatalogVoice {
    pub voice_id: String,
    pub display_name: String,
    pub description: String,
    pub manifest_sha256: String,
    pub references: Vec<TtsCatalogReference>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TtsCatalog {
    pub voices: Vec<TtsCatalogVoice>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TtsComposed {
    pub plan_digest: String,
    pub utterance_count: usize,
    pub chunk_count: usize,
    pub engine: String,
    pub plan_path: PathBuf,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TtsValidated {
    pub plan_digest: String,
    pub utterance_count: usize,
    pub chunk_count: usize,
    pub engine: String,
}

impl TtsWorkerClient {
    pub async fn capabilities(&self, project: &ProjectId) -> Result<TtsCapabilities, PortError> {
        let event = self
            .preflight(
                project,
                "preflight",
                "capabilities",
                PreflightOperation::Capabilities,
                json!({}),
            )
            .await?;
        let capabilities: TtsCapabilities = decode(event.payload)?;
        if !capabilities.protocol_versions.contains(&1)
            || !capabilities.speech_plan_versions.contains(&1)
            || !capabilities
                .operations
                .iter()
                .any(|operation| operation == "execute")
            || !capabilities
                .operations
                .iter()
                .any(|operation| operation == "cancel")
        {
            return Err(invalid(
                "Installed TTS worker is incompatible with this adapter",
            ));
        }
        Ok(capabilities)
    }

    pub async fn catalog(&self, project: &ProjectId) -> Result<TtsCatalog, PortError> {
        let event = self
            .preflight(
                project,
                "preflight",
                "catalog",
                PreflightOperation::Catalog,
                json!({"voice_store": self.runtime().voice_store}),
            )
            .await?;
        decode(event.payload)
    }

    pub async fn compose(
        &self,
        project: &ProjectId,
        run_id: &str,
        attempt_id: &str,
        draft_path: &Path,
        plan_path: &Path,
        profile_id: &str,
    ) -> Result<TtsComposed, PortError> {
        let runtime = self.runtime();
        confined(&runtime.managed_root, draft_path)?;
        confined(&runtime.managed_root, plan_path)?;
        if !draft_path.is_file() || draft_path == plan_path {
            return Err(invalid(
                "TTS plan draft is unavailable or overlaps its output",
            ));
        }
        let profile_path = profile_path(&runtime.profile_root, profile_id)?;
        let event = self
            .preflight(
                project,
                run_id,
                attempt_id,
                PreflightOperation::Compose,
                json!({
                    "draft_path": draft_path,
                    "voice_store": runtime.voice_store,
                    "profile_path": profile_path,
                    "plan_path": plan_path,
                }),
            )
            .await?;
        let composed: TtsComposed = decode(event.payload)?;
        if composed.plan_path != plan_path
            || composed.utterance_count == 0
            || composed.chunk_count < composed.utterance_count
            || !hex_sha256(&composed.plan_digest)
        {
            return Err(invalid("TTS worker returned an invalid composed plan"));
        }
        Ok(composed)
    }

    pub async fn validate_plan(
        &self,
        project: &ProjectId,
        run_id: &str,
        attempt_id: &str,
        plan_path: &Path,
        profile_id: &str,
    ) -> Result<TtsValidated, PortError> {
        let runtime = self.runtime();
        confined(&runtime.managed_root, plan_path)?;
        if !plan_path.is_file() {
            return Err(invalid("TTS plan is unavailable"));
        }
        let profile_path = profile_path(&runtime.profile_root, profile_id)?;
        let event = self
            .preflight(
                project,
                run_id,
                attempt_id,
                PreflightOperation::Validate,
                json!({
                    "plan_path": plan_path,
                    "voice_store": runtime.voice_store,
                    "profile_path": profile_path,
                }),
            )
            .await?;
        let validated: TtsValidated = decode(event.payload)?;
        if validated.utterance_count == 0
            || validated.chunk_count < validated.utterance_count
            || !hex_sha256(&validated.plan_digest)
        {
            return Err(invalid("TTS worker returned invalid plan coverage"));
        }
        Ok(validated)
    }
}

fn profile_path(root: &Path, id: &str) -> Result<PathBuf, PortError> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(invalid("TTS profile ID is invalid"));
    }
    let path = root.join(format!("{id}.json"));
    confined(root, &path)?;
    if !path.is_file() {
        return Err(invalid("TTS profile is not installed"));
    }
    Ok(path)
}

fn hex_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> Result<T, PortError> {
    serde_json::from_value(value).map_err(|error| invalid(&error.to_string()))
}

fn invalid(message: &str) -> PortError {
    PortError::InvalidSource {
        message: message.into(),
    }
}
