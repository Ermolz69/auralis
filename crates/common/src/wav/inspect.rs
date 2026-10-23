use super::{Pcm16Error, WavProperties};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub fn inspect_pcm16_wav(path: &Path) -> Result<WavProperties, Pcm16Error> {
    inspect_pcm16(&mut File::open(path)?)
}

pub fn inspect_pcm16(reader: &mut (impl Read + Seek)) -> Result<WavProperties, Pcm16Error> {
    let length = reader.seek(SeekFrom::End(0))?;
    reader.seek(SeekFrom::Start(0))?;
    let mut riff = [0_u8; 12];
    reader.read_exact(&mut riff)?;
    if &riff[..4] != b"RIFF"
        || &riff[8..] != b"WAVE"
        || u64::from(u32::from_le_bytes([riff[4], riff[5], riff[6], riff[7]])) + 8 != length
    {
        return Err(Pcm16Error::Invalid("WAV is not a complete RIFF WAVE file"));
    }
    let mut format = None;
    let mut data = None;
    let mut chunks = 0;
    while reader.stream_position()? + 8 <= length {
        chunks += 1;
        if chunks > 4096 {
            return Err(Pcm16Error::Invalid("WAV chunk count exceeds 4096"));
        }
        let mut header = [0_u8; 8];
        reader.read_exact(&mut header)?;
        let size = u64::from(u32::from_le_bytes([
            header[4], header[5], header[6], header[7],
        ]));
        let content = reader.stream_position()?;
        let next = content
            .checked_add(size + (size & 1))
            .filter(|end| *end <= length)
            .ok_or(Pcm16Error::Invalid("WAV chunk extends beyond the file"))?;
        match &header[..4] {
            b"fmt " if format.is_none() && size >= 16 => {
                let mut fields = [0_u8; 16];
                reader.read_exact(&mut fields)?;
                let encoding = u16::from_le_bytes([fields[0], fields[1]]);
                let channels = u16::from_le_bytes([fields[2], fields[3]]);
                let sample_rate = u32::from_le_bytes([fields[4], fields[5], fields[6], fields[7]]);
                let byte_rate = u32::from_le_bytes([fields[8], fields[9], fields[10], fields[11]]);
                let align = u16::from_le_bytes([fields[12], fields[13]]);
                let depth = u16::from_le_bytes([fields[14], fields[15]]);
                if encoding != 1
                    || channels == 0
                    || sample_rate == 0
                    || depth != 16
                    || channels.checked_mul(2) != Some(align)
                    || u64::from(byte_rate) != u64::from(sample_rate) * u64::from(align)
                {
                    return Err(Pcm16Error::Invalid("WAV format is not valid 16-bit PCM"));
                }
                format = Some((sample_rate, channels, align));
            }
            b"data" if data.is_none() => data = Some((content, size)),
            b"fmt " | b"data" => return Err(Pcm16Error::Invalid("WAV repeats a required chunk")),
            _ => {}
        }
        reader.seek(SeekFrom::Start(next))?;
    }
    if reader.stream_position()? != length {
        return Err(Pcm16Error::Invalid("WAV has trailing incomplete bytes"));
    }
    let (sample_rate, channels, align) =
        format.ok_or(Pcm16Error::Invalid("WAV lacks its format chunk"))?;
    let (data_offset, data_bytes) = data.ok_or(Pcm16Error::Invalid("WAV lacks audio data"))?;
    if data_bytes == 0 || data_bytes % u64::from(align) != 0 {
        return Err(Pcm16Error::Invalid("WAV has incomplete audio frames"));
    }
    let frames = data_bytes / u64::from(align);
    let numerator = frames
        .checked_mul(1000)
        .ok_or(Pcm16Error::Invalid("WAV duration overflows"))?;
    let rate = u64::from(sample_rate);
    let whole = numerator / rate;
    let remainder = numerator % rate;
    let duration_ms =
        whole + u64::from(remainder * 2 > rate || (remainder * 2 == rate && whole % 2 == 1));
    Ok(WavProperties {
        sample_rate,
        channels,
        frames,
        duration_ms,
        data_offset,
        data_bytes,
    })
}
