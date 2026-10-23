use super::SpeechAudioLayout;
use domain::speech::TimingMode;

#[derive(Debug, Clone)]
pub struct SpeechAudioAssemblyRequest {
    pub source: SpeechAudioLayout,
    pub mode: TimingMode,
    pub source_duration_ms: Option<u64>,
    pub pause_after_ms: Vec<u64>,
}
