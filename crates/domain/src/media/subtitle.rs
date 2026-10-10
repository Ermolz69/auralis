#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubtitleTrack {
    pub id: String,
    pub language: String,
    pub label: Option<String>,
    pub format: Option<String>,
    pub is_auto_generated: bool,
}
