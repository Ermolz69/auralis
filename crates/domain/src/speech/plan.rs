use super::identity::{Sha256Digest, SpeechId};
use super::voice::{CastSnapshot, VoiceSnapshot};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeechPlan {
    pub version: u8,
    pub plan_id: SpeechId,
    pub project_id: SpeechId,
    pub source_sha256: Sha256Digest,
    pub text_revision_id: SpeechId,
    pub review: ReviewDecision,
    pub timing_mode: TimingMode,
    pub cast: CastSnapshot,
    pub cast_sha256: Sha256Digest,
    pub voices: Vec<VoiceSnapshot>,
    pub utterances: Vec<Utterance>,
    pub plan_sha256: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewDecision {
    pub mode: ReviewMode,
    pub decision_id: SpeechId,
    pub language: SpeechLanguage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewMode {
    ReviewedSubtitle,
    ReviewedPlainText,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimingMode {
    Continuous,
    SubtitleWindow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeechLanguage {
    #[serde(rename = "ru")]
    Russian,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Utterance {
    pub utterance_id: SpeechId,
    pub source: UtteranceSource,
    pub text: String,
    pub role_id: SpeechId,
    pub voice_id: SpeechId,
    pub reference_id: SpeechId,
    pub engine_id: SpeechId,
    pub emotion_id: SpeechId,
    pub profile_id: SpeechId,
    pub start_ms: Option<u64>,
    pub end_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UtteranceSource {
    SubtitleCue { cue_id: SpeechId },
    TextRange { start: u64, end: u64 },
}
