use super::{control::WorkControl, error, pcm};
use common::wav::WavProperties;
use ports::{PortError, speech_audio::UtteranceAudioRange};
use std::{
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

#[cfg(test)]
mod tests;

pub(super) fn extract(
    input: &mut (impl Read + Seek),
    properties: &WavProperties,
    range: &UtteranceAudioRange,
    target: &Path,
    control: &WorkControl,
) -> Result<u64, PortError> {
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .map_err(error::io)?;
    let bytes = write_range(input, properties, range, &mut output, control)?;
    output.flush().map_err(error::io)?;
    Ok(bytes)
}

pub(super) fn write_range(
    input: &mut (impl Read + Seek),
    properties: &WavProperties,
    range: &UtteranceAudioRange,
    output: &mut impl Write,
    control: &WorkControl,
) -> Result<u64, PortError> {
    control.check()?;
    if properties.sample_rate != 48000
        || !matches!(properties.channels, 1 | 2)
        || range.frame_count == 0
        || range
            .first_frame
            .checked_add(range.frame_count)
            .is_none_or(|end| end > properties.frames)
    {
        return Err(error::invalid(
            "Utterance PCM range is outside the verified 48 kHz WAV",
        ));
    }
    let align = u64::from(properties.channels) * 2;
    let bytes = range
        .frame_count
        .checked_mul(align)
        .filter(|value| *value <= (1_u64 << 32) - 128)
        .ok_or_else(|| error::invalid("Utterance PCM range size overflow"))?;
    let offset = range
        .first_frame
        .checked_mul(align)
        .and_then(|value| value.checked_add(properties.data_offset))
        .ok_or_else(|| error::invalid("Utterance PCM range offset overflow"))?;
    pcm::write_header(output, bytes, properties.channels)?;
    input.seek(SeekFrom::Start(offset)).map_err(error::io)?;
    let mut remaining = bytes;
    let mut buffer = [0_u8; 65_536];
    while remaining > 0 {
        control.check()?;
        let size = remaining.min(buffer.len() as u64) as usize;
        input.read_exact(&mut buffer[..size]).map_err(error::io)?;
        output.write_all(&buffer[..size]).map_err(error::io)?;
        remaining -= size as u64;
    }
    Ok(bytes + 44)
}
