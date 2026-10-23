use super::{control::WorkControl, error};
use common::wav::{WavProperties, inspect_pcm16_wav};
use ports::PortError;
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

const BLOCK: usize = 65_536;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct PcmStats {
    pub peak: u16,
    pub full_scale: u64,
    pub samples: u64,
}

pub(super) fn hash_file(path: &Path, control: &WorkControl) -> Result<String, PortError> {
    let mut file = File::open(path).map_err(error::io)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; BLOCK];
    loop {
        control.check()?;
        let read = file.read(&mut buffer).map_err(error::io)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

pub(super) fn copy_verified(
    source: &Path,
    target: &Path,
    expected: &str,
    limit: u64,
    control: &WorkControl,
) -> Result<u64, PortError> {
    let mut input = File::open(source).map_err(error::io)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .map_err(error::io)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; BLOCK];
    let mut bytes = 0_u64;
    loop {
        control.check()?;
        let read = input.read(&mut buffer).map_err(error::io)?;
        if read == 0 {
            break;
        }
        bytes = bytes
            .checked_add(read as u64)
            .filter(|size| *size <= limit)
            .ok_or_else(|| error::invalid("Audio source byte budget exceeded"))?;
        output.write_all(&buffer[..read]).map_err(error::io)?;
        digest.update(&buffer[..read]);
    }
    if format!("{:x}", digest.finalize()) != expected {
        return Err(error::invalid(
            "Utterance audio hash differs from immutable facts",
        ));
    }
    output.flush().map_err(error::io)?;
    Ok(bytes)
}

pub(super) fn append_pcm(
    path: &Path,
    digest: &mut Sha256,
    stats: &mut PcmStats,
    control: &WorkControl,
) -> Result<WavProperties, PortError> {
    let metadata = inspect_pcm16_wav(path).map_err(error::wave)?;
    let mut file = File::open(path).map_err(error::io)?;
    file.seek(SeekFrom::Start(metadata.data_offset))
        .map_err(error::io)?;
    let mut remaining = metadata.data_bytes;
    let mut buffer = [0_u8; BLOCK];
    while remaining > 0 {
        control.check()?;
        let size = remaining.min(BLOCK as u64) as usize;
        file.read_exact(&mut buffer[..size]).map_err(error::io)?;
        digest.update(&buffer[..size]);
        for pair in buffer[..size].chunks_exact(2) {
            let sample = i16::from_le_bytes([pair[0], pair[1]]);
            stats.peak = stats.peak.max(sample.unsigned_abs());
            stats.full_scale += u64::from(sample == i16::MIN || sample == i16::MAX);
            stats.samples += 1;
        }
        remaining -= size as u64;
    }
    Ok(metadata)
}

pub(super) fn write_silence(
    path: &Path,
    frames: u64,
    channels: u16,
    digest: &mut Sha256,
    stats: &mut PcmStats,
    control: &WorkControl,
) -> Result<(), PortError> {
    let bytes = frames
        .checked_mul(u64::from(channels) * 2)
        .filter(|bytes| *bytes <= (1_u64 << 32) - 128)
        .ok_or_else(|| error::invalid("Silence exceeds PCM16 RIFF limit"))?;
    let size = u32::try_from(bytes).map_err(|_| error::invalid("Silence size overflow"))?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(error::io)?;
    let mut header = Vec::with_capacity(44);
    header.extend_from_slice(b"RIFF");
    header.extend_from_slice(&(size + 36).to_le_bytes());
    header.extend_from_slice(b"WAVEfmt ");
    header.extend_from_slice(&16_u32.to_le_bytes());
    header.extend_from_slice(&1_u16.to_le_bytes());
    header.extend_from_slice(&channels.to_le_bytes());
    header.extend_from_slice(&48000_u32.to_le_bytes());
    header.extend_from_slice(&(48000_u32 * u32::from(channels) * 2).to_le_bytes());
    header.extend_from_slice(&(channels * 2).to_le_bytes());
    header.extend_from_slice(&16_u16.to_le_bytes());
    header.extend_from_slice(b"data");
    header.extend_from_slice(&size.to_le_bytes());
    output.write_all(&header).map_err(error::io)?;
    let zeros = [0_u8; BLOCK];
    let mut remaining = bytes;
    while remaining > 0 {
        control.check()?;
        let size = remaining.min(BLOCK as u64) as usize;
        output.write_all(&zeros[..size]).map_err(error::io)?;
        digest.update(&zeros[..size]);
        remaining -= size as u64;
    }
    stats.samples += bytes / 2;
    output.flush().map_err(error::io)
}
