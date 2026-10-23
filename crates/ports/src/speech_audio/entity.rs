use super::UtteranceAudioRange;
use domain::speech::Sha256Digest;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeechAudioLayout {
    pub aggregate_path: PathBuf,
    pub aggregate_sha256: Sha256Digest,
    pub source_plan_sha256: Sha256Digest,
    pub raw_manifest_sha256: Sha256Digest,
    pub assembly_policy: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub frame_count: u64,
    pub utterances: Vec<UtteranceAudioRange>,
}
