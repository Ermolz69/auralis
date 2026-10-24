use domain::speech::Sha256Digest;
use ports::audio_inspection::SpeechTimingSettings;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InspectAssemblyRequest {
    pub source_run_id: Uuid,
    pub settings: SpeechTimingSettings,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrepareAssemblyRequest {
    pub source: InspectAssemblyRequest,
    pub expected_preview_sha256: Sha256Digest,
    pub reviewed: bool,
}
