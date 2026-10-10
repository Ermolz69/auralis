use adapters_caster::CasterRuntimeConfig;
use ports::caster::ReferenceDetails;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::bootstrap::paths::AppPaths;
use crate::dto::error::CommandError;

const MAX_REFERENCE_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Serialize)]
pub struct ReferencePlaybackDto {
    pub voice_id: String,
    pub reference_id: String,
    pub engine_id: String,
    pub emotion_id: String,
    pub transcript: String,
    pub audio_sha256: String,
    pub qualification_status: String,
    pub qualification_report_sha256: Option<String>,
    pub playback_path: String,
}

pub async fn prepare_reference_playback(
    app: &AppHandle,
    reference: ReferenceDetails,
) -> Result<ReferencePlaybackDto, CommandError> {
    let runtime = app
        .try_state::<CasterRuntimeConfig>()
        .ok_or_else(|| CommandError::Validation("Caster runtime is unavailable".into()))?;
    let paths = app.state::<AppPaths>();
    let source = PathBuf::from(&reference.audio_path);
    let store = runtime.voice_store.clone();
    let cache = paths.caster_audio_cache();
    let expected = reference.audio_sha256.clone();
    let prepared = tauri::async_runtime::spawn_blocking(move || {
        copy_verified(&store, &source, &cache, &expected)
    })
    .await
    .map_err(|_| CommandError::Io("Audio preparation failed".into()))??;
    app.asset_protocol_scope()
        .allow_file(&prepared)
        .map_err(|_| CommandError::Io("Audio playback permission failed".into()))?;
    Ok(ReferencePlaybackDto {
        voice_id: reference.voice_id,
        reference_id: reference.reference_id,
        engine_id: reference.engine_id,
        emotion_id: reference.emotion_id,
        transcript: reference.transcript,
        audio_sha256: reference.audio_sha256,
        qualification_status: reference.qualification_status,
        qualification_report_sha256: reference.qualification_report_sha256,
        playback_path: prepared.to_string_lossy().into_owned(),
    })
}

fn copy_verified(
    store: &Path,
    source: &Path,
    cache: &Path,
    expected: &str,
) -> Result<PathBuf, CommandError> {
    if expected.len() != 64
        || !expected
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(CommandError::Validation("Invalid reference digest".into()));
    }
    let store = store
        .canonicalize()
        .map_err(|_| CommandError::Validation("Voice library is unavailable".into()))?;
    let source = source
        .canonicalize()
        .map_err(|_| CommandError::Validation("Reference audio is unavailable".into()))?;
    if !source.starts_with(&store) || source == store {
        return Err(CommandError::Validation(
            "Reference audio is outside the voice library".into(),
        ));
    }
    let metadata = fs::symlink_metadata(&source)
        .map_err(|_| CommandError::Validation("Reference audio is unavailable".into()))?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_REFERENCE_BYTES
    {
        return Err(CommandError::Validation(
            "Reference audio is unsupported".into(),
        ));
    }
    fs::create_dir_all(cache).map_err(|_| CommandError::Io("Audio cache is unavailable".into()))?;
    let target = cache.join(format!("{expected}.wav"));
    if target.exists() {
        verify_file(&target, expected)?;
        return Ok(target);
    }
    let temporary = cache.join(format!(".{}.wav", Uuid::new_v4()));
    let copy_result = (|| {
        let mut input = File::open(&source)
            .map_err(|_| CommandError::Io("Reference audio cannot be opened".into()))?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| CommandError::Io("Audio cache staging failed".into()))?;
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 65536];
        let mut total = 0_u64;
        loop {
            let count = input
                .read(&mut buffer)
                .map_err(|_| CommandError::Io("Reference audio read failed".into()))?;
            if count == 0 {
                break;
            }
            total += count as u64;
            if total > MAX_REFERENCE_BYTES {
                return Err(CommandError::Validation(
                    "Reference audio is too large".into(),
                ));
            }
            hasher.update(&buffer[..count]);
            output
                .write_all(&buffer[..count])
                .map_err(|_| CommandError::Io("Audio cache write failed".into()))?;
        }
        if format!("{:x}", hasher.finalize()) != expected {
            return Err(CommandError::Validation(
                "Reference audio changed after inspection".into(),
            ));
        }
        output
            .sync_all()
            .map_err(|_| CommandError::Io("Audio cache sync failed".into()))?;
        if target.exists() {
            verify_file(&target, expected)?;
        } else {
            fs::rename(&temporary, &target)
                .map_err(|_| CommandError::Io("Audio cache publication failed".into()))?;
        }
        Ok(target.clone())
    })();
    if temporary.exists() {
        let _ = fs::remove_file(&temporary);
    }
    copy_result
}

fn verify_file(path: &Path, expected: &str) -> Result<(), CommandError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| CommandError::Io("Cached audio is unavailable".into()))?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_REFERENCE_BYTES
    {
        return Err(CommandError::Validation("Cached audio is invalid".into()));
    }
    let mut file =
        File::open(path).map_err(|_| CommandError::Io("Cached audio cannot be opened".into()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 65536];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| CommandError::Io("Cached audio read failed".into()))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    if format!("{:x}", hasher.finalize()) != expected {
        return Err(CommandError::Validation(
            "Cached audio digest differs".into(),
        ));
    }
    Ok(())
}
