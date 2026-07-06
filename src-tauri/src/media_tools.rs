use std::path::PathBuf;
use tauri::AppHandle;

pub fn resolve_ffprobe_candidates(_app: &AppHandle) -> Vec<PathBuf> {
    // TODO: Connect Tauri externalBin API later
    vec![
        PathBuf::from("src-tauri/binaries/ffprobe-x86_64-pc-windows-msvc.exe"),
        PathBuf::from("src-tauri/binaries/ffprobe-aarch64-apple-darwin"),
        PathBuf::from("ffprobe"),
        PathBuf::from("ffprobe.exe"),
    ]
}

pub fn resolve_ytdlp_candidates(_app: &AppHandle) -> Vec<PathBuf> {
    // TODO: Connect Tauri externalBin API later
    vec![
        PathBuf::from("src-tauri/binaries/yt-dlp-x86_64-pc-windows-msvc.exe"),
        PathBuf::from("src-tauri/binaries/yt-dlp_macos"),
        PathBuf::from("yt-dlp"),
        PathBuf::from("yt-dlp.exe"),
    ]
}
