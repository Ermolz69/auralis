use crate::speech_audio::SpeechAudioLayout;
use domain::speech::timeline::{AudioTimeline, AudioTimelineRequest, TimelineConflict};

#[derive(Debug, Clone)]
pub struct SpeechAudioInspection {
    pub source: SpeechAudioLayout,
    pub timeline_request: AudioTimelineRequest,
    pub timeline: Option<AudioTimeline>,
    pub conflicts: Vec<TimelineConflict>,
}
