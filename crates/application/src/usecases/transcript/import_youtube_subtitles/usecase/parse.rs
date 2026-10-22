use super::*;
use domain::transcript::{Transcript, TranscriptSegment, TranscriptSegmentId};
use ports::translation_inspection::TranslationSourceFormat;
use sha2::{Digest, Sha256};

impl ImportYoutubeSubtitlesUseCase {
    pub(super) async fn parse_srt(
        &self,
        path: &std::path::Path,
        content: &str,
        language: &str,
    ) -> Result<Transcript, ApplicationError> {
        let inspector =
            self.inspector
                .as_ref()
                .ok_or_else(|| ApplicationError::InvalidOperation {
                    message: "Strict SRT inspection is not configured".into(),
                })?;
        let inspected = inspector
            .inspect(path, TranslationSourceFormat::Srt)
            .await?;
        if inspected.source_sha256 != format!("{:x}", Sha256::digest(content.as_bytes())) {
            return Err(ApplicationError::InvalidOperation {
                message: "Subtitle bytes changed during inspection".into(),
            });
        }
        Ok(Transcript {
            language: language.into(),
            segments: inspected
                .segments
                .into_iter()
                .map(|cue| TranscriptSegment {
                    id: TranscriptSegmentId::new(),
                    index: cue.ordinal,
                    start_ms: cue.start_ms,
                    end_ms: cue.end_ms,
                    source_text: cue.source_lines.join("\n"),
                    translated_text: None,
                    adapted_text: None,
                    speaker: None,
                    confidence: None,
                })
                .collect(),
        })
    }
}
