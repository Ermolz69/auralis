use ports::{
    audio_inspection::SpeechAudioInspection,
    audio_jobs::{AudioAssemblyIntent, AudioAssemblyPlan},
};

pub(super) struct ComputedAssembly {
    pub intent: AudioAssemblyIntent,
    pub plan: AudioAssemblyPlan,
    pub inspection: SpeechAudioInspection,
    pub profile_id: String,
}
