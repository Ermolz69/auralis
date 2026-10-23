use super::MeasuredUtterance;
use crate::speech::{Sha256Digest, SpeechId, TimingMode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AudioTimelineRequest {
    pub source_plan_sha256: Sha256Digest,
    pub raw_manifest_sha256: Sha256Digest,
    pub mode: TimingMode,
    pub expected_utterance_ids: Vec<SpeechId>,
    pub utterances: Vec<MeasuredUtterance>,
    pub source_duration_ms: Option<u64>,
}
