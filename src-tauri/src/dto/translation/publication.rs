use ports::translation_links::{TranslationPublicationRecord, TranslationPublicationState};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationPublicationDto {
    pub translation_id: String,
    pub result_id: String,
    pub run_id: String,
    pub artifact_id: String,
    pub source_sha256: String,
    pub needs_review: bool,
    pub state: &'static str,
}

impl From<TranslationPublicationRecord> for TranslationPublicationDto {
    fn from(value: TranslationPublicationRecord) -> Self {
        Self {
            translation_id: value.translation_id.to_string(),
            result_id: value.result_id.to_string(),
            run_id: value.run_id.to_string(),
            artifact_id: value.artifact_id.to_string(),
            source_sha256: value.source_sha256,
            needs_review: value.needs_review,
            state: match value.state {
                TranslationPublicationState::Pending => "pending",
                TranslationPublicationState::Ready => "ready",
                TranslationPublicationState::Failed => "failed",
            },
        }
    }
}
