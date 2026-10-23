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
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .map_err(error::io)?;
    pcm::write_header(&mut output, bytes, properties.channels)?;
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
    output.flush().map_err(error::io)?;
    Ok(bytes + 44)
}
