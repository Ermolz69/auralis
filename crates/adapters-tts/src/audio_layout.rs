use crate::{
    TtsCompleted, TtsExecutionRequest,
    result_verification::{array, hash_file_cancellable, invalid, number, string, verified_result},
    verification_control::check_cancel,
};
use common::wav::inspect_pcm16_wav;
use domain::speech::{Sha256Digest, SpeechId, timeline::SourceWindow};
use ports::{
    PortError,
    cancellation::CancellationToken,
    speech_audio::{SpeechAudioLayout, UtteranceAudioRange},
};
use std::{collections::HashSet, path::Path};

pub(crate) const AUDIO_LAYOUT_POLICY: &str = "pcm16_stream_fade10ms_peak089125_kaiser5_v1";
const RATES: [u32; 13] = [
    8000, 11025, 12000, 16000, 22050, 24000, 32000, 44100, 48000, 88200, 96000, 176400, 192000,
];

pub fn verify_result_audio_layout(
    request: &TtsExecutionRequest,
    completed: &TtsCompleted,
) -> Result<SpeechAudioLayout, PortError> {
    verify_result_audio_layout_cancellable(request, completed, &CancellationToken::new())
}

pub(crate) fn verify_result_audio_layout_cancellable(
    request: &TtsExecutionRequest,
    completed: &TtsCompleted,
    cancel: &CancellationToken,
) -> Result<SpeechAudioLayout, PortError> {
    let (plan, manifest, plan_sha256) = verified_result(request, completed, cancel)?;
    if manifest
        .get("audio_assembly")
        .and_then(serde_json::Value::as_str)
        != Some(AUDIO_LAYOUT_POLICY)
    {
        return Err(invalid(
            "Install the verified streaming TTS package and render a new result before audio assembly",
        ));
    }
    let utterances = array(&plan, "utterances")?;
    let segments = array(&manifest, "segments")?;
    if utterances.is_empty()
        || utterances.len() > 8192
        || segments.len() > 8192
        || completed.sample_rate != 48000
        || !matches!(completed.channels, 1 | 2)
    {
        return Err(invalid(
            "Audio layout exceeds supported coverage or PCM properties",
        ));
    }
    let mut ranges = Vec::with_capacity(utterances.len());
    let mut ids = HashSet::new();
    let mut cursor = 0;
    let mut offset = 0_u64;
    for utterance in utterances {
        check_cancel(cancel)?;
        let id = SpeechId::parse(string(utterance, "utterance_id")?).map_err(invalid)?;
        if !ids.insert(id.clone()) {
            return Err(invalid("Audio layout repeats an utterance"));
        }
        let mut chunk_ids = Vec::new();
        let mut frames = 0_u64;
        while let Some(segment) = segments.get(cursor) {
            if string(segment, "utterance_id")? != id.as_str() {
                break;
            }
            let path = Path::new(string(segment, "audio")?);
            let wav = inspect_pcm16_wav(path).map_err(|error| invalid(&error.to_string()))?;
            if !RATES.contains(&wav.sample_rate)
                || wav.channels != completed.channels
                || wav.frames == 0
                || hash_file_cancellable(path, cancel)? != string(segment, "audio_sha256")?
            {
                return Err(invalid(
                    "Technical chunk PCM properties or identity changed",
                ));
            }
            let count = wav
                .frames
                .checked_mul(48000)
                .map(|value| value.div_ceil(u64::from(wav.sample_rate)))
                .ok_or_else(|| invalid("Technical chunk frame count overflow"))?;
            frames = frames
                .checked_add(count)
                .ok_or_else(|| invalid("Utterance frame count overflow"))?;
            chunk_ids.push(string(segment, "chunk_id")?.to_string());
            cursor += 1;
        }
        let source_window = match (utterance.get("start_ms"), utterance.get("end_ms")) {
            (Some(start), Some(end)) if start.is_null() && end.is_null() => None,
            (None, None) => None,
            _ => Some(SourceWindow {
                start_ms: number(utterance, "start_ms")?,
                end_ms: number(utterance, "end_ms")?,
            }),
        };
        ranges.push(UtteranceAudioRange {
            utterance_id: id,
            first_frame: offset,
            frame_count: frames,
            chunk_ids,
            source_window,
        });
        offset = offset
            .checked_add(frames)
            .ok_or_else(|| invalid("Aggregate frame count overflow"))?;
    }
    if cursor != segments.len() || offset != completed.frames {
        return Err(invalid(
            "Technical chunk frames do not cover the complete aggregate WAV",
        ));
    }
    Ok(SpeechAudioLayout {
        aggregate_path: completed.output.clone(),
        aggregate_sha256: Sha256Digest::parse(&completed.output_sha256).map_err(invalid)?,
        source_plan_sha256: Sha256Digest::parse(plan_sha256).map_err(invalid)?,
        raw_manifest_sha256: Sha256Digest::parse(&completed.manifest_sha256).map_err(invalid)?,
        assembly_policy: AUDIO_LAYOUT_POLICY.into(),
        sample_rate: completed.sample_rate,
        channels: completed.channels,
        frame_count: completed.frames,
        utterances: ranges,
    })
}
