use ports::error::PortError;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub struct WavProperties {
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: u64,
    pub duration_ms: u64,
}

pub fn inspect_pcm16_wav(path: &Path) -> Result<WavProperties, PortError> {
    let mut file = File::open(path).map_err(io_error)?;
    let length = file.metadata().map_err(io_error)?.len();
    let mut riff = [0u8; 12];
    file.read_exact(&mut riff).map_err(io_error)?;
    if &riff[..4] != b"RIFF"
        || &riff[8..] != b"WAVE"
        || u32::from_le_bytes([riff[4], riff[5], riff[6], riff[7]]) as u64 + 8 != length
    {
        return Err(invalid("TTS output is not a complete RIFF WAVE file"));
    }
    let mut format = None;
    let mut data_bytes = None;
    while file.stream_position().map_err(io_error)? + 8 <= length {
        let mut header = [0u8; 8];
        file.read_exact(&mut header).map_err(io_error)?;
        let size = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as u64;
        let content = file.stream_position().map_err(io_error)?;
        let padded = size + (size & 1);
        let next = content
            .checked_add(padded)
            .filter(|end| *end <= length)
            .ok_or_else(|| invalid("TTS WAV chunk extends beyond the file"))?;
        match &header[..4] {
            b"fmt " if format.is_none() && size >= 16 => {
                let mut fields = [0u8; 16];
                file.read_exact(&mut fields).map_err(io_error)?;
                let encoding = u16::from_le_bytes([fields[0], fields[1]]);
                let channels = u16::from_le_bytes([fields[2], fields[3]]);
                let sample_rate = u32::from_le_bytes([fields[4], fields[5], fields[6], fields[7]]);
                let byte_rate = u32::from_le_bytes([fields[8], fields[9], fields[10], fields[11]]);
                let block_align = u16::from_le_bytes([fields[12], fields[13]]);
                let bit_depth = u16::from_le_bytes([fields[14], fields[15]]);
                if encoding != 1
                    || channels == 0
                    || sample_rate == 0
                    || bit_depth != 16
                    || block_align != channels.saturating_mul(2)
                    || u64::from(byte_rate) != u64::from(sample_rate) * u64::from(block_align)
                {
                    return Err(invalid("TTS WAV format is not valid 16-bit PCM"));
                }
                format = Some((sample_rate, channels, block_align));
            }
            b"data" if data_bytes.is_none() => data_bytes = Some(size),
            b"fmt " | b"data" => return Err(invalid("TTS WAV repeats a required chunk")),
            _ => {}
        }
        file.seek(SeekFrom::Start(next)).map_err(io_error)?;
    }
    if file.stream_position().map_err(io_error)? != length {
        return Err(invalid("TTS WAV has trailing incomplete bytes"));
    }
    let (sample_rate, channels, block_align) =
        format.ok_or_else(|| invalid("TTS WAV lacks its format chunk"))?;
    let data_bytes = data_bytes.ok_or_else(|| invalid("TTS WAV lacks audio data"))?;
    if data_bytes == 0 || data_bytes % u64::from(block_align) != 0 {
        return Err(invalid("TTS WAV has incomplete audio frames"));
    }
    let frames = data_bytes / u64::from(block_align);
    let duration_numerator = frames
        .checked_mul(1000)
        .ok_or_else(|| invalid("TTS WAV duration overflows"))?;
    let divisor = u64::from(sample_rate);
    let whole = duration_numerator / divisor;
    let remainder = duration_numerator % divisor;
    let duration_ms = if remainder * 2 > divisor || (remainder * 2 == divisor && whole % 2 == 1) {
        whole + 1
    } else {
        whole
    };
    Ok(WavProperties {
        sample_rate,
        channels,
        frames,
        duration_ms,
    })
}

fn io_error(error: std::io::Error) -> PortError {
    PortError::Io {
        message: error.to_string(),
    }
}

fn invalid(message: &str) -> PortError {
    PortError::InvalidStoredData {
        entity_type: "tts_result".into(),
        entity_id: "wav".into(),
        field: "audio".into(),
        message: message.into(),
    }
}
