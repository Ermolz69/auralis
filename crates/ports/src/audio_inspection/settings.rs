use domain::speech::TimingMode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeechTimingSettings {
    pub mode: TimingMode,
    pub source_duration_ms: Option<u64>,
    pub pause_after_ms: Vec<u64>,
}
