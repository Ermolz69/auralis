#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MediaSource {
    ManagedLocalFile {
        artifact_id: super::ArtifactId,
        original_filename: String,
    },
    #[serde(alias = "LocalFile")]
    ExternalLocalFile {
        path: String,
    },
    YoutubeUrl {
        url: String,
    },
    RemoteUrl {
        url: String,
    },
}
