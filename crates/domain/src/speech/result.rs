use super::identity::{Sha256Digest, SpeechId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SegmentResult {
    pub version: u8,
    pub plan_sha256: Sha256Digest,
    pub run_id: SpeechId,
    pub attempt_id: SpeechId,
    pub utterance_id: SpeechId,
    pub voice_id: SpeechId,
    pub reference_id: SpeechId,
    pub profile_id: SpeechId,
    pub model_sha256: Sha256Digest,
    pub audio_sha256: Sha256Digest,
    pub sample_rate: u32,
    pub channels: u16,
    pub frame_count: u64,
    pub duration_ms: u64,
    pub chunks: Vec<AudioChunk>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioChunk {
    pub chunk_id: SpeechId,
    pub text_sha256: Sha256Digest,
    pub audio_sha256: Sha256Digest,
    pub frame_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SynthesisResult {
    pub version: u8,
    pub plan_sha256: Sha256Digest,
    pub run_id: SpeechId,
    pub attempt_id: SpeechId,
    pub profile_sha256: Sha256Digest,
    pub model_sha256: Sha256Digest,
    pub expected_utterance_ids: Vec<SpeechId>,
    pub segments: Vec<SegmentResult>,
    pub warnings: Vec<String>,
    pub outcome: SynthesisOutcome,
    pub result_sha256: Sha256Digest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SynthesisOutcome {
    Complete,
}
