#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MediaSource {
    LocalFile { path: String },
    YoutubeUrl { url: String },
    RemoteUrl { url: String },
}
