use super::{
    config::AudioAssemblyLimits,
    control::WorkControl,
    error,
    input::{self, AssemblyInput},
    pcm, slicing,
    staging::PreparedAssembly,
};
use domain::speech::{
    Sha256Digest,
    timeline::{AudioTimelineRequest, MeasuredUtterance},
};
use ports::{
    PortError,
    audio_assembly::{AudioAssemblyRequest, UtteranceAudioFile},
    speech_audio::{SpeechAudioAssemblyRequest, SpeechAudioLayout},
};
use std::{
    collections::HashSet,
    fs::{self, File},
    path::Path,
};

#[cfg(test)]
mod tests;

const POLICY: &str = "pcm16_stream_fade10ms_peak089125_kaiser5_v1";

pub(super) fn prepare(
    request: SpeechAudioAssemblyRequest,
    input_root: &Path,
    stage_root: &Path,
    limits: &AudioAssemblyLimits,
    control: &WorkControl,
) -> Result<PreparedAssembly, PortError> {
    control.check()?;
    validate(&request.source)?;
    if request.pause_after_ms.len() != request.source.utterances.len() {
        return Err(error::invalid(
            "Specify one explicit pause for every complete utterance",
        ));
    }
    let path = &request.source.aggregate_path;
    let metadata = fs::symlink_metadata(path).map_err(error::io)?;
    let canonical = fs::canonicalize(path).map_err(error::io)?;
    if !path.is_absolute()
        || !metadata.is_file()
        || metadata.file_type().is_symlink()
        || !canonical.starts_with(input_root)
    {
        return Err(error::invalid(
            "Aggregate audio is outside the owned input root or is a link",
        ));
    }
    let directory = tempfile::Builder::new()
        .prefix("speech-extraction-")
        .tempdir_in(stage_root)
        .map_err(error::io)?;
    let snapshot = directory.path().join("aggregate.wav");
    let mut bytes = pcm::copy_verified(
        &canonical,
        &snapshot,
        request.source.aggregate_sha256.as_str(),
        limits.max_input_bytes,
        control,
    )?;
    let properties = common::wav::inspect_pcm16_wav(&snapshot).map_err(error::wave)?;
    if properties.sample_rate != request.source.sample_rate
        || properties.channels != request.source.channels
        || properties.frames != request.source.frame_count
    {
        return Err(error::invalid(
            "Aggregate WAV properties differ from verified source layout",
        ));
    }
    let mut aggregate = File::open(&snapshot).map_err(error::io)?;
    let mut inputs = Vec::with_capacity(request.source.utterances.len());
    let mut measured = Vec::with_capacity(request.source.utterances.len());
    for (index, range) in request.source.utterances.iter().enumerate() {
        control.check()?;
        let required = range.frame_count * u64::from(properties.channels) * 2 + 44;
        if required > limits.max_input_bytes.saturating_sub(bytes) {
            return Err(error::invalid(
                "Aggregate snapshot and extraction exceed the input byte budget",
            ));
        }
        let target = directory.path().join(format!("utterance-{index:05}.wav"));
        bytes += slicing::extract(&mut aggregate, &properties, range, &target, control)?;
        measured.push(MeasuredUtterance {
            utterance_id: range.utterance_id.clone(),
            audio_sha256: Sha256Digest::parse(pcm::hash_file(&target, control)?)
                .map_err(error::invalid)?,
            sample_rate: properties.sample_rate,
            channels: properties.channels,
            frame_count: range.frame_count,
            source_window: range.source_window.clone(),
            pause_after_ms: request.pause_after_ms[index],
        });
        inputs.push(UtteranceAudioFile {
            utterance_id: range.utterance_id.clone(),
            path: target,
        });
    }
    let assembly = AudioAssemblyRequest {
        timeline: AudioTimelineRequest {
            source_plan_sha256: request.source.source_plan_sha256.clone(),
            raw_manifest_sha256: request.source.raw_manifest_sha256.clone(),
            mode: request.mode,
            expected_utterance_ids: request
                .source
                .utterances
                .iter()
                .map(|range| range.utterance_id.clone())
                .collect(),
            utterances: measured,
            source_duration_ms: request.source_duration_ms,
        },
        inputs,
    };
    let root = fs::canonicalize(directory.path()).map_err(error::io)?;
    let mut prepared = input::prepare(
        AssemblyInput::Files(assembly),
        &root,
        stage_root,
        limits,
        control,
    )?;
    prepared.raw_source = Some(request.source);
    Ok(prepared)
}

pub(super) fn validate(source: &SpeechAudioLayout) -> Result<(), PortError> {
    if source.assembly_policy != POLICY
        || source.sample_rate != 48000
        || !matches!(source.channels, 1 | 2)
        || source.frame_count == 0
        || source.utterances.is_empty()
        || source.utterances.len() > 8192
        || source.frame_count > ((1_u64 << 32) - 128) / (u64::from(source.channels) * 2)
    {
        return Err(error::invalid(
            "Unsupported TTS aggregate layout; render with the verified streaming package",
        ));
    }
    let mut offset = 0_u64;
    let mut ids = HashSet::new();
    let mut chunk_count = 0_usize;
    for range in &source.utterances {
        if range.first_frame != offset
            || range.frame_count == 0
            || range.chunk_ids.is_empty()
            || !ids.insert(&range.utterance_id)
        {
            return Err(error::invalid(
                "Utterance ranges must cover the aggregate exactly once in order",
            ));
        }
        for (index, id) in range.chunk_ids.iter().enumerate() {
            if *id != format!("{}-chunk-{:04}", range.utterance_id.as_str(), index + 1) {
                return Err(error::invalid("Technical chunk identity or order changed"));
            }
        }
        chunk_count = chunk_count
            .checked_add(range.chunk_ids.len())
            .filter(|count| *count <= 8192)
            .ok_or_else(|| error::invalid("Technical chunk count exceeds the bridge limit"))?;
        offset = offset
            .checked_add(range.frame_count)
            .filter(|frames| *frames <= source.frame_count)
            .ok_or_else(|| error::invalid("Utterance ranges exceed the aggregate"))?;
    }
    if offset != source.frame_count {
        return Err(error::invalid("Utterance ranges omit aggregate audio"));
    }
    Ok(())
}
