use async_trait::async_trait;
use domain::project::ProjectId;
use serde::Serialize;

use crate::error::PortError;

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct SpeechProfileOption {
    pub profile_id: String,
    pub profile_sha256: String,
    pub engine_id: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct SpeechReferenceOption {
    pub reference_id: String,
    pub engine_id: String,
    pub emotion_id: String,
    pub transcript: String,
    pub audio_sha256: String,
    pub qualification_status: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct SpeechVoiceOption {
    pub voice_id: String,
    pub display_name: String,
    pub description: String,
    pub manifest_sha256: String,
    pub references: Vec<SpeechReferenceOption>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct SpeechCatalogSnapshot {
    pub profiles: Vec<SpeechProfileOption>,
    pub voices: Vec<SpeechVoiceOption>,
}

#[async_trait]
pub trait SpeechCatalogPort: Send + Sync {
    async fn snapshot(&self, project: &ProjectId) -> Result<SpeechCatalogSnapshot, PortError>;
}
