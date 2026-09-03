#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum YoutubeImportState {
    Downloading,
    Staged,
    Failed,
}
