mod planner;

use super::{Sha256Digest, SpeechId, TimingMode};
use serde::{Deserialize, Serialize};

pub use planner::plan_audio_timeline;

pub const ASSEMBLY_SAMPLE_RATE: u32 = 48_000;
pub const MAX_ASSEMBLY_UTTERANCES: usize = 8192;
pub const AUDIO_TIMELINE_POLICY: &str = "preserve_full_speech_48k_v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceWindow {
    pub start_ms: u64,
    pub end_ms: u64,
}

/// One complete untimed utterance, measured after verifying audio and manifest
/// hashes. The adapter must first verify and combine every technical chunk;
/// substituting one chunk for the entire utterance is not valid input.
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AudioTimeline {
    pub version: u8,
    pub policy: &'static str,
    pub source_plan_sha256: Sha256Digest,
    pub raw_manifest_sha256: Sha256Digest,
    pub mode: TimingMode,
    pub sample_rate: u32,
    pub channels: u16,
    pub frame_count: u64,
    pub spans: Vec<TimelineSpan>,
}

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
