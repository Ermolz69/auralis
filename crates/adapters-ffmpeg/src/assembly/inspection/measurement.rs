use super::super::{control::WorkControl, error, slicing};
use common::wav::WavProperties;
use domain::speech::{
    Sha256Digest,
    timeline::{AudioTimelineRequest, MeasuredUtterance},
};
use ports::{PortError, audio_inspection::SpeechTimingSettings, speech_audio::SpeechAudioLayout};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{self, Write},
};

struct HashSink(Sha256);

impl Write for HashSink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(super) fn measure(
    file: &mut File,
    properties: &WavProperties,
    layout: &SpeechAudioLayout,
    settings: SpeechTimingSettings,
    control: &WorkControl,
) -> Result<AudioTimelineRequest, PortError> {
    if settings.pause_after_ms.len() != layout.utterances.len() {
        return Err(error::invalid(
            "Specify one explicit pause per complete utterance",
        ));
    }
    let mut utterances = Vec::with_capacity(layout.utterances.len());
    for (index, range) in layout.utterances.iter().enumerate() {
        control.check()?;
        let mut sink = HashSink(Sha256::new());
        slicing::write_range(file, properties, range, &mut sink, control)?;
        utterances.push(MeasuredUtterance {
            utterance_id: range.utterance_id.clone(),
            audio_sha256: Sha256Digest::parse(format!("{:x}", sink.0.finalize()))
                .map_err(error::invalid)?,
            sample_rate: properties.sample_rate,
            channels: properties.channels,
            frame_count: range.frame_count,
            source_window: range.source_window.clone(),
            pause_after_ms: settings.pause_after_ms[index],
        });
    }
    Ok(AudioTimelineRequest {
        source_plan_sha256: layout.source_plan_sha256.clone(),
        raw_manifest_sha256: layout.raw_manifest_sha256.clone(),
        mode: settings.mode,
        source_duration_ms: settings.source_duration_ms,
        expected_utterance_ids: layout
            .utterances
            .iter()
            .map(|range| range.utterance_id.clone())
            .collect(),
        utterances,
    })
}
