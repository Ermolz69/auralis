#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Manager;

pub fn resolve_ffprobe_candidates_internal(resource_dir: Option<PathBuf>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    // 1. Packaged resource path
    if let Some(dir) = resource_dir {
        candidates.push(
            dir.join("binaries")
                .join("ffprobe-x86_64-pc-windows-msvc.exe"),
        );
        candidates.push(dir.join("binaries").join("ffprobe-aarch64-apple-darwin"));
        candidates.push(dir.join("binaries").join("ffprobe"));
        candidates.push(dir.join("binaries").join("ffprobe.exe"));
    }

    // 2. Fallbacks for dev environment
    candidates.push(PathBuf::from(
        "src-tauri/binaries/ffprobe-x86_64-pc-windows-msvc.exe",
    ));
    candidates.push(PathBuf::from(
        "src-tauri/binaries/ffprobe-aarch64-apple-darwin",
    ));
    candidates.push(PathBuf::from("ffprobe"));
    candidates.push(PathBuf::from("ffprobe.exe"));

    candidates
}

pub fn resolve_ytdlp_candidates_internal(resource_dir: Option<PathBuf>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    // 1. Packaged resource path
    if let Some(dir) = resource_dir {
        candidates.push(
            dir.join("binaries")
                .join("yt-dlp-x86_64-pc-windows-msvc.exe"),
        );
        candidates.push(dir.join("binaries").join("yt-dlp_macos"));
        candidates.push(dir.join("binaries").join("yt-dlp"));
        candidates.push(dir.join("binaries").join("yt-dlp.exe"));
    }

    // 2. Fallbacks for dev environment
    candidates.push(PathBuf::from(
        "src-tauri/binaries/yt-dlp-x86_64-pc-windows-msvc.exe",
    ));
    candidates.push(PathBuf::from("src-tauri/binaries/yt-dlp_macos"));
    candidates.push(PathBuf::from("yt-dlp"));
    candidates.push(PathBuf::from("yt-dlp.exe"));

    candidates
}

pub fn resolve_ffprobe_candidates(app: &AppHandle) -> Vec<PathBuf> {
    resolve_ffprobe_candidates_internal(app.path().resource_dir().ok())
}

pub fn resolve_ytdlp_candidates(app: &AppHandle) -> Vec<PathBuf> {
    resolve_ytdlp_candidates_internal(app.path().resource_dir().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_ffprobe_with_resource_dir() {
        let dir = PathBuf::from("/mock/resource/dir");
        let candidates = resolve_ffprobe_candidates_internal(Some(dir));

        assert_eq!(candidates.len(), 8);
        assert_eq!(
            candidates[0],
            PathBuf::from("/mock/resource/dir/binaries/ffprobe-x86_64-pc-windows-msvc.exe")
        );
        assert_eq!(
            candidates[4],
            PathBuf::from("src-tauri/binaries/ffprobe-x86_64-pc-windows-msvc.exe")
        );
    }

    #[test]
    fn test_resolve_ytdlp_without_resource_dir() {
        let candidates = resolve_ytdlp_candidates_internal(None);

        assert_eq!(candidates.len(), 4);
        assert_eq!(
            candidates[0],
            PathBuf::from("src-tauri/binaries/yt-dlp-x86_64-pc-windows-msvc.exe")
        );
    }
}
