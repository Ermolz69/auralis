use super::super::{control::WorkControl, error};
use ports::{PortError, audio_inspection::ManagedAudioInput};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::Path,
};

pub(super) const MAX_JSON_BYTES: u64 = 16 * 1024 * 1024;

pub(super) fn open_owned(
    input: &ManagedAudioInput,
    root: &Path,
    limit: u64,
) -> Result<File, PortError> {
    let metadata = fs::symlink_metadata(&input.path).map_err(error::io)?;
    let canonical = fs::canonicalize(&input.path).map_err(error::io)?;
    if !input.path.is_absolute()
        || !metadata.is_file()
        || metadata.file_type().is_symlink()
        || !canonical.starts_with(root)
        || metadata.len() != input.size_bytes
        || input.size_bytes == 0
        || input.size_bytes > limit
    {
        return Err(error::invalid(
            "Ready speech input is outside owned storage, linked, changed or over its byte limit",
        ));
    }
    File::open(canonical).map_err(error::io)
}

pub(super) fn verify(
    file: &mut File,
    input: &ManagedAudioInput,
    collect: bool,
    control: &WorkControl,
) -> Result<Vec<u8>, PortError> {
    file.seek(SeekFrom::Start(0)).map_err(error::io)?;
    let mut bytes = Vec::new();
    let mut digest = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 65_536];
    loop {
        control.check()?;
        let read = file.read(&mut buffer).map_err(error::io)?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(read as u64)
            .filter(|bytes| *bytes <= input.size_bytes)
            .ok_or_else(|| error::invalid("Ready speech input grew beyond its immutable size"))?;
        digest.update(&buffer[..read]);
        if collect {
            bytes.extend_from_slice(&buffer[..read]);
        }
    }
    if total != input.size_bytes || format!("{:x}", digest.finalize()) != input.sha256.as_str() {
        return Err(error::invalid(
            "Ready speech input hash or size differs from its publication",
        ));
    }
    Ok(bytes)
}

pub(super) fn json<T: serde::de::DeserializeOwned>(
    input: &ManagedAudioInput,
    root: &Path,
    control: &WorkControl,
) -> Result<T, PortError> {
    let mut file = open_owned(input, root, MAX_JSON_BYTES)?;
    let bytes = verify(&mut file, input, true, control)?;
    serde_json::from_slice(&bytes).map_err(|error| {
        super::super::error::invalid(format!("Ready speech metadata is invalid: {error}"))
    })
}
