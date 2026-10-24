use super::{ManagedAudioInput, SpeechTimingSettings};
use domain::speech::{Sha256Digest, SpeechId};

#[derive(Debug, Clone)]
pub struct SpeechAudioInspectionRequest {
    pub aggregate: ManagedAudioInput,
    pub layout: ManagedAudioInput,
    pub manifest: ManagedAudioInput,
    pub source_plan_sha256: Sha256Digest,
    pub expected_utterance_ids: Vec<SpeechId>,
    pub settings: SpeechTimingSettings,
}
