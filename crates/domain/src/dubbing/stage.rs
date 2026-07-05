#[derive(Debug, Clone, PartialEq, Eq)]
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
