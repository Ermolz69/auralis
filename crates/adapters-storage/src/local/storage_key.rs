use domain::media::ArtifactKind;
use domain::project::ProjectId;
use domain::media::ArtifactId;

pub fn kind_slug(kind: &ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::SourceVideo => "source-video",
        ArtifactKind::DownloadedVideo => "downloaded-video",
        ArtifactKind::ExtractedAudio => "extracted-audio",
        ArtifactKind::OriginalSubtitle => "original-subtitle",
        ArtifactKind::GeneratedTranscript => "generated-transcript",
        ArtifactKind::NormalizedTranscript => "normalized-transcript",
        ArtifactKind::TranslatedTranscript => "translated-transcript",
        ArtifactKind::GeneratedSpeechSegment => "generated-speech-segment",
        ArtifactKind::MixedAudio => "mixed-audio",
        ArtifactKind::PreviewVideo => "preview-video",
        ArtifactKind::FinalVideo => "final-video",
        ArtifactKind::LogFile => "log-file",
    }
}

pub fn make_storage_key(
    project_id: &ProjectId,
    artifact_id: &ArtifactId,
    kind: &ArtifactKind,
    extension: &str,
) -> String {
    let slug = kind_slug(kind);
    format!("{}/{}/{}.{}", project_id.0, slug, artifact_id.0, extension)
}
