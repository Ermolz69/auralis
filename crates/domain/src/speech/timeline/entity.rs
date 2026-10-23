use super::TimelineSpan;
use crate::speech::{Sha256Digest, TimingMode};
use serde::Serialize;

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
