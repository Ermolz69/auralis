use domain::media::ArtifactId;
use domain::media::ArtifactKind;
use domain::project::ProjectId;

pub fn kind_slug(kind: &ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::SourceVideo => "source-video",
        ArtifactKind::DownloadedVideo => "downloaded-video",
        ArtifactKind::ExtractedAudio => "extracted-audio",
        ArtifactKind::OriginalSubtitle => "original-subtitle",
        ArtifactKind::GeneratedTranscript => "generated-transcript",
        ArtifactKind::NormalizedTranscript => "generated-transcript",
        ArtifactKind::TranslatedTranscript => "translated-transcript",
        ArtifactKind::GeneratedSpeechSegment => "generated-speech-segment",
        ArtifactKind::MixedAudio => "mixed-audio",
        ArtifactKind::PreviewVideo => "preview-video",
        ArtifactKind::FinalVideo => "final-video",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_kinds_use_only_final_project_directories() {
        let cases = [
            (ArtifactKind::SourceVideo, "source-video"),
            (ArtifactKind::DownloadedVideo, "downloaded-video"),
            (ArtifactKind::ExtractedAudio, "extracted-audio"),
            (ArtifactKind::OriginalSubtitle, "original-subtitle"),
            (ArtifactKind::GeneratedTranscript, "generated-transcript"),
            (ArtifactKind::NormalizedTranscript, "generated-transcript"),
            (ArtifactKind::TranslatedTranscript, "translated-transcript"),
            (
                ArtifactKind::GeneratedSpeechSegment,
                "generated-speech-segment",
            ),
            (ArtifactKind::MixedAudio, "mixed-audio"),
            (ArtifactKind::PreviewVideo, "preview-video"),
            (ArtifactKind::FinalVideo, "final-video"),
        ];

        for (kind, expected) in cases {
            assert_eq!(kind_slug(&kind), expected);
        }
    }
}
