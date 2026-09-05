use domain::media::{Artifact, ArtifactKind, ArtifactLocation, ArtifactState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactDto {
    pub id: String,
    pub kind: ArtifactKindDto,
    pub location: ArtifactLocationDto,
    pub size_bytes: Option<u64>,
    pub state: ArtifactStateDto,
    pub created_at: String,
    pub updated_at: String,
    pub ready_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactKindDto {
    SourceVideo,
    DownloadedVideo,
    ExtractedAudio,
    OriginalSubtitle,
    GeneratedTranscript,
    NormalizedTranscript,
    TranslatedTranscript,
    GeneratedSpeechSegment,
    MixedAudio,
    PreviewVideo,
    FinalVideo,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum ArtifactLocationDto {
    LocalPath(String),
    StorageKey(String),
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactStateDto {
    PendingFinalize,
    Ready,
    Deleting,
    Failed,
}

impl From<&Artifact> for ArtifactDto {
    fn from(artifact: &Artifact) -> Self {
        Self {
            id: artifact.id.to_string(),
            kind: (&artifact.kind).into(),
            location: (&artifact.location).into(),
            size_bytes: artifact.size_bytes,
            state: (&artifact.state).into(),
            created_at: artifact.created_at.to_rfc3339(),
            updated_at: artifact.updated_at.to_rfc3339(),
            ready_at: artifact.ready_at.as_ref().map(chrono::DateTime::to_rfc3339),
        }
    }
}

impl From<&ArtifactKind> for ArtifactKindDto {
    fn from(kind: &ArtifactKind) -> Self {
        match kind {
            ArtifactKind::SourceVideo => Self::SourceVideo,
            ArtifactKind::DownloadedVideo => Self::DownloadedVideo,
            ArtifactKind::ExtractedAudio => Self::ExtractedAudio,
            ArtifactKind::OriginalSubtitle => Self::OriginalSubtitle,
            ArtifactKind::GeneratedTranscript => Self::GeneratedTranscript,
            ArtifactKind::NormalizedTranscript => Self::NormalizedTranscript,
            ArtifactKind::TranslatedTranscript => Self::TranslatedTranscript,
            ArtifactKind::GeneratedSpeechSegment => Self::GeneratedSpeechSegment,
            ArtifactKind::MixedAudio => Self::MixedAudio,
            ArtifactKind::PreviewVideo => Self::PreviewVideo,
            ArtifactKind::FinalVideo => Self::FinalVideo,
        }
    }
}

impl From<ArtifactKindDto> for ArtifactKind {
    fn from(kind: ArtifactKindDto) -> Self {
        match kind {
            ArtifactKindDto::SourceVideo => Self::SourceVideo,
            ArtifactKindDto::DownloadedVideo => Self::DownloadedVideo,
            ArtifactKindDto::ExtractedAudio => Self::ExtractedAudio,
            ArtifactKindDto::OriginalSubtitle => Self::OriginalSubtitle,
            ArtifactKindDto::GeneratedTranscript => Self::GeneratedTranscript,
            ArtifactKindDto::NormalizedTranscript => Self::NormalizedTranscript,
            ArtifactKindDto::TranslatedTranscript => Self::TranslatedTranscript,
            ArtifactKindDto::GeneratedSpeechSegment => Self::GeneratedSpeechSegment,
            ArtifactKindDto::MixedAudio => Self::MixedAudio,
            ArtifactKindDto::PreviewVideo => Self::PreviewVideo,
            ArtifactKindDto::FinalVideo => Self::FinalVideo,
        }
    }
}

impl From<&ArtifactLocation> for ArtifactLocationDto {
    fn from(location: &ArtifactLocation) -> Self {
        match location {
            ArtifactLocation::LocalPath(path) => Self::LocalPath(path.clone()),
            ArtifactLocation::StorageKey(key) => Self::StorageKey(key.clone()),
        }
    }
}

impl From<&ArtifactState> for ArtifactStateDto {
    fn from(state: &ArtifactState) -> Self {
        match state {
            ArtifactState::PendingFinalize => Self::PendingFinalize,
            ArtifactState::Ready => Self::Ready,
            ArtifactState::Deleting => Self::Deleting,
            ArtifactState::Failed => Self::Failed,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn artifact_uses_explicit_camel_case_wire_shape() {
        let artifact = Artifact {
            id: domain::media::ArtifactId::new(),
            kind: ArtifactKind::DownloadedVideo,
            location: ArtifactLocation::StorageKey("project/source/video.mp4".to_string()),
            size_bytes: Some(42),
            state: ArtifactState::Ready,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            ready_at: None,
        };

        let value = serde_json::to_value(ArtifactDto::from(&artifact)).unwrap();

        assert_eq!(value["kind"], "downloadedVideo");
        assert_eq!(value["location"]["kind"], "storageKey");
        assert_eq!(value["location"]["value"], "project/source/video.mp4");
        assert_eq!(value["sizeBytes"], 42);
        assert!(value.get("size_bytes").is_none());
    }
}
