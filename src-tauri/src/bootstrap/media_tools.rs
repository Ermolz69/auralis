use std::path::PathBuf;

use tauri::{AppHandle, Manager};

pub fn resolve_ffmpeg_candidates_internal(resource_dir: Option<PathBuf>) -> Vec<PathBuf> {
    resolve_candidates_internal(resource_dir, "ffmpeg")
}

pub fn resolve_ffprobe_candidates_internal(resource_dir: Option<PathBuf>) -> Vec<PathBuf> {
    resolve_candidates_internal(resource_dir, "ffprobe")
}

pub fn resolve_ytdlp_candidates_internal(resource_dir: Option<PathBuf>) -> Vec<PathBuf> {
    resolve_candidates_internal(resource_dir, "yt-dlp")
}

pub fn resolve_ffmpeg_candidates(app: &AppHandle) -> Vec<PathBuf> {
    resolve_ffmpeg_candidates_internal(app.path().resource_dir().ok())
}

pub fn resolve_ffprobe_candidates(app: &AppHandle) -> Vec<PathBuf> {
    resolve_ffprobe_candidates_internal(app.path().resource_dir().ok())
}

pub fn resolve_ytdlp_candidates(app: &AppHandle) -> Vec<PathBuf> {
    resolve_ytdlp_candidates_internal(app.path().resource_dir().ok())
}

fn resolve_candidates_internal(resource_dir: Option<PathBuf>, tool: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(filename) = bundled_filename(tool) {
        if let Some(dir) = resource_dir {
            candidates.push(dir.join("binaries").join(&filename));
        }

        candidates.push(PathBuf::from("src-tauri/binaries").join(filename));
    }

    candidates.push(PathBuf::from(tool));
    if cfg!(windows) {
        candidates.push(PathBuf::from(format!("{tool}.exe")));
    }

    candidates
}

fn bundled_filename(tool: &str) -> Option<String> {
    let target = bundled_target()?;
    let extension = if cfg!(windows) { ".exe" } else { "" };
    Some(format!("{tool}-{target}{extension}"))
}

fn bundled_target() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Some("x86_64-pc-windows-msvc"),
        ("linux", "x86_64") => Some("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64") => Some("aarch64-unknown-linux-gnu"),
        ("macos", "x86_64") => Some("x86_64-apple-darwin"),
        ("macos", "aarch64") => Some("aarch64-apple-darwin"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packaged_candidate_precedes_development_and_path_fallbacks() {
        let Some(filename) = bundled_filename("ffprobe") else {
            panic!("the test runner target must have a bundled media-tools mapping");
        };
        let resource_dir = PathBuf::from("mock-resource-dir");

        let candidates = resolve_ffprobe_candidates_internal(Some(resource_dir.clone()));

        assert_eq!(candidates[0], resource_dir.join("binaries").join(&filename));
        assert_eq!(
            candidates[1],
            PathBuf::from("src-tauri/binaries").join(filename)
        );
        assert!(candidates.contains(&PathBuf::from("ffprobe")));
    }

    #[test]
    fn all_media_tools_share_the_same_resolution_policy() {
        for tool in ["ffmpeg", "ffprobe", "yt-dlp"] {
            let candidates = resolve_candidates_internal(None, tool);

            assert!(
                candidates
                    .first()
                    .is_some_and(|path| path.starts_with("src-tauri/binaries"))
            );
            assert!(candidates.contains(&PathBuf::from(tool)));
        }
    }
}
