use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DubbingPipelineStage {
    ValidateSource,
    InspectSubtitles,
    FetchMetadata,
    DownloadMedia,
    ExtractOrGenerateTranscript,
    SegmentTranscript,
    TranslateTranscript,
    PrepareDubbingScript,
    SynthesizeSegments,
    PostprocessAudio,
    MuxAudioTrack,
    ExportResult,
}
