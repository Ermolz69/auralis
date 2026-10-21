use super::identity::{Sha256Digest, SpeechId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CastSnapshot {
    pub version: u8,
    pub roles: BTreeMap<SpeechId, CastRole>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CastRole {
    pub display_name: String,
    pub voice_id: Option<SpeechId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceSnapshot {
    pub version: u8,
    pub voice_manifest_version: u8,
    pub voice_id: SpeechId,
    pub manifest_sha256: Sha256Digest,
    pub references: Vec<VoiceReferenceSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoiceReferenceSnapshot {
    pub reference_id: SpeechId,
    pub engine_id: SpeechId,
    pub emotion_id: SpeechId,
    pub audio_sha256: Sha256Digest,
    pub transcript_sha256: Sha256Digest,
    pub qualification_status: QualificationStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationStatus {
    Unverified,
    Qualified,
    Rejected,
}
