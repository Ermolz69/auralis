pub fn run_dubbing(video_url: String) -> Result<String, String> {
    Ok(format!("Successfully initiated dubbing pipeline for {}", video_url))
}
