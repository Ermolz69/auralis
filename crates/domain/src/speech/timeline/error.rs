use crate::speech::SpeechId;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TimelineConflict {
    InvalidCount {
        actual: usize,
        max: usize,
    },
    CoverageMismatch,
    DuplicateUtterance {
        utterance_id: SpeechId,
    },
    InvalidAudio {
        utterance_id: SpeechId,
    },
    ChannelMismatch {
        utterance_id: SpeechId,
        expected: u16,
        actual: u16,
    },
    MissingMediaDuration,
    MissingWindow {
        utterance_id: SpeechId,
    },
    InvalidWindow {
        utterance_id: SpeechId,
        start_ms: u64,
        end_ms: u64,
    },
    OutsideMedia {
        utterance_id: SpeechId,
        end_ms: u64,
        duration_ms: u64,
    },
    OverlappingWindows {
        previous_utterance_id: SpeechId,
        utterance_id: SpeechId,
    },
    ImpossibleFit {
        utterance_id: SpeechId,
        required_frames: u64,
        available_frames: u64,
    },
    PauseInSubtitleWindow {
        utterance_id: SpeechId,
    },
    ArithmeticOverflow,
    RiffSizeLimit {
        frames: u64,
        channels: u16,
    },
}
