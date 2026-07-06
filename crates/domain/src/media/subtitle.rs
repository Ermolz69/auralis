#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleTrack {
    pub id: String,
    pub language: String,
    pub label: Option<String>,
    pub format: Option<String>,
    pub is_auto_generated: bool,
}
