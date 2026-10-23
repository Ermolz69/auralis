use super::{config::AudioAssemblyLimits, control::WorkControl, error, source, staging};
use domain::speech::timeline::plan_audio_timeline;
use ports::{
    PortError, audio_assembly::AudioAssemblyRequest, speech_audio::SpeechAudioAssemblyRequest,
};
use std::path::Path;

pub(super) enum AssemblyInput {
    Files(AudioAssemblyRequest),
    Speech(SpeechAudioAssemblyRequest),
}

pub(super) fn prepare(
    input: AssemblyInput,
    input_root: &Path,
    stage_root: &Path,
    limits: &AudioAssemblyLimits,
    control: &WorkControl,
) -> Result<staging::PreparedAssembly, PortError> {
    let request = match input {
        AssemblyInput::Files(request) => request,
        AssemblyInput::Speech(request) => {
            return source::prepare(request, input_root, stage_root, limits, control);
        }
    };
    let timeline = plan_audio_timeline(&request.timeline).map_err(|conflicts| {
        error::invalid(format!(
            "Resolve {} timing conflicts before audio assembly",
            conflicts.len()
        ))
    })?;
    if request.inputs.len() != request.timeline.utterances.len()
        || request
            .inputs
            .iter()
            .zip(&request.timeline.utterances)
            .any(|(input, facts)| {
                input.utterance_id != facts.utterance_id || facts.sample_rate != 48000
            })
    {
        return Err(error::invalid(
            "Assembly requires exact ordered complete 48 kHz utterance audio",
        ));
    }
    if timeline.frame_count * u64::from(timeline.channels) * 2 > limits.max_pcm_bytes {
        return Err(error::invalid(
            "Timeline exceeds the configured output byte budget",
        ));
    }
    staging::prepare(&request, timeline, input_root, stage_root, limits, control)
}
