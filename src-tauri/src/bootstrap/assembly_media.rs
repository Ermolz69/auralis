use domain::speech::Sha256Digest;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};

pub(super) const WINDOWS_FFMPEG_SHA256: &str =
    "4044b3924c977ad31229d504c5d5b8685f9553124fbaff6e9c99048b42830341";
pub(super) const WINDOWS_FFPROBE_SHA256: &str =
    "fc37ca23d31ee08bb8f7e108edf3822f6ef3efc1a8d306bbe0b779190230710b";

pub(super) fn pinned_ffprobe(app: &AppHandle) -> Result<(PathBuf, Sha256Digest), String> {
    let (ffmpeg, _) = pinned_ffmpeg(app)?;
    let path = ffmpeg.with_file_name("ffprobe-x86_64-pc-windows-msvc.exe");
    verify(&path, WINDOWS_FFPROBE_SHA256)?;
    Ok((
        path,
        Sha256Digest::parse(WINDOWS_FFPROBE_SHA256).map_err(str::to_string)?,
    ))
}

pub(super) fn pinned_ffmpeg(app: &AppHandle) -> Result<(PathBuf, Sha256Digest), String> {
    if !cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        return Err("Для этой платформы ещё не закреплена сборка FFmpeg".into());
    }
    let filename = "ffmpeg-x86_64-pc-windows-msvc.exe";
    let mut candidates = Vec::new();
    if let Ok(root) = app.path().resource_dir() {
        candidates.push(root.join("binaries").join(filename));
    }
    if cfg!(debug_assertions) {
        candidates.push(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("binaries")
                .join(filename),
        );
    }
    let selected = candidates
        .into_iter()
        .find(|path| path.exists())
        .ok_or("FFmpeg отсутствует; восстановите установку Auralis")?;
    verify(&selected, WINDOWS_FFMPEG_SHA256)?;
    Ok((
        fs::canonicalize(selected).map_err(|error| error.to_string())?,
        Sha256Digest::parse(WINDOWS_FFMPEG_SHA256).map_err(str::to_string)?,
    ))
}

fn verify(path: &Path, expected: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !path.is_absolute()
        || !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() == 0
        || metadata.len() > 256 * 1024 * 1024
    {
        return Err("Недопустимый файл управляемого FFmpeg; восстановите установку Auralis".into());
    }
    let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 65536];
    let mut read = 0_u64;
    loop {
        let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        read += count as u64;
        if read > metadata.len() {
            return Err("Файл FFmpeg изменился во время проверки".into());
        }
        digest.update(&buffer[..count]);
    }
    if read != metadata.len() || format!("{:x}", digest.finalize()) != expected {
        return Err("Контрольная сумма FFmpeg не совпадает; восстановите установку Auralis".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn managed_tool_pin_refuses_changed_empty_and_relative_files()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let file = temp.path().join("fixture.exe");
        fs::write(&file, b"fixture executable")?;
        let hash = format!("{:x}", Sha256::digest(b"fixture executable"));
        assert!(verify(&file, &hash).is_ok());
        fs::write(&file, b"changed executable")?;
        assert!(verify(&file, &hash).is_err());
        fs::write(&file, b"")?;
        assert!(verify(&file, &hash).is_err());
        assert!(verify(Path::new("ffmpeg.exe"), &hash).is_err());
        Ok(())
    }
}
