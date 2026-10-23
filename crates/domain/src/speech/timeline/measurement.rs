use super::SourceWindow;
use crate::speech::{Sha256Digest, SpeechId};
use serde::{Deserialize, Serialize};

/// Every technical chunk must be verified and combined before measuring this
/// complete untimed utterance. A single chunk cannot stand in for the utterance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeasuredUtterance {
    pub utterance_id: SpeechId,
    pub audio_sha256: Sha256Digest,
    pub sample_rate: u32,
    pub channels: u16,
    pub frame_count: u64,
    pub source_window: Option<SourceWindow>,
    pub pause_after_ms: u64,
}
