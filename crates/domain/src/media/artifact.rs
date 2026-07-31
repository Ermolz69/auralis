use crate::transcript::TranscriptSegmentId;
use uuid::Uuid;

use crate::error::DomainError;
use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ArtifactId(pub Uuid);

impl ArtifactId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ArtifactId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for ArtifactId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ArtifactId {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::from_str(s)
            .map(ArtifactId)
            .map_err(|_| DomainError::ValidationError(format!("Invalid ArtifactId: {}", s)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ArtifactKind {
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
    LogFile,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ArtifactLocation {
    LocalPath(String),
    StorageKey(String),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactState {
    PendingFinalize,
    Ready,
    Deleting,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Artifact {
    pub id: ArtifactId,
    pub kind: ArtifactKind,
    pub location: ArtifactLocation,
    pub size_bytes: Option<u64>,
    pub state: ArtifactState,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub ready_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SynthesizedSegment {
    pub segment_id: TranscriptSegmentId,
    pub audio_artifact: Artifact,
    pub duration_ms: Option<u64>,
}
