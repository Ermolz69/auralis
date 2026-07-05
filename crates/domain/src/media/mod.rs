use uuid::Uuid;
use crate::transcript::TranscriptSegmentId;

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
pub enum MediaSource {
    LocalFile { path: String },
    YoutubeUrl { url: String },
    RemoteUrl { url: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MediaMetadata {
    pub duration_ms: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<f32>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub audio_channels: Option<u8>,
    pub sample_rate: Option<u32>,
    pub container: Option<String>,
    pub has_video: bool,
    pub has_audio: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtitleTrack {
    pub id: String,
    pub language: String,
    pub label: Option<String>,
    pub format: Option<String>,
    pub is_auto_generated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynthesizedSegment {
    pub segment_id: TranscriptSegmentId,
    pub audio_artifact: Artifact,
    pub duration_ms: Option<u64>,
}

