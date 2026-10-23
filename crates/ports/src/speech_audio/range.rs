use domain::speech::{SpeechId, timeline::SourceWindow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UtteranceAudioRange {
    pub utterance_id: SpeechId,
    pub first_frame: u64,
    pub frame_count: u64,
    pub chunk_ids: Vec<String>,
    pub source_window: Option<SourceWindow>,
}
