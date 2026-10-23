use super::SourceWindow;
use crate::speech::{Sha256Digest, SpeechId};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TimelineSpan {
    Silence {
        start_frame: u64,
        frame_count: u64,
    },
    Speech {
        utterance_id: SpeechId,
        audio_sha256: Sha256Digest,
        raw_sample_rate: u32,
        raw_frame_count: u64,
        source_window: Option<SourceWindow>,
        start_frame: u64,
        frame_count: u64,
    },
}
