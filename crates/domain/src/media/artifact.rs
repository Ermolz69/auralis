use crate::transcript::TranscriptSegmentId;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ArtifactId(pub Uuid);

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactLocation {
    LocalPath(String),
    StorageKey(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    pub id: ArtifactId,
    pub kind: ArtifactKind,
    pub location: ArtifactLocation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynthesizedSegment {
    pub segment_id: TranscriptSegmentId,
    pub audio_artifact: Artifact,
    pub duration_ms: Option<u64>,
}
