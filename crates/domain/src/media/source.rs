#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaSource {
    LocalFile { path: String },
    YoutubeUrl { url: String },
    RemoteUrl { url: String },
}
