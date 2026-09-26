use ports::translation_links::TranslationPublicationRecord;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationHistoryEntryDto {
    pub result_id: String,
    pub result_revision: u32,
    pub run_id: String,
    pub artifact_id: String,
    pub needs_review: bool,
}

impl From<TranslationPublicationRecord> for TranslationHistoryEntryDto {
    fn from(value: TranslationPublicationRecord) -> Self {
        Self {
            result_id: value.result_id.to_string(),
            result_revision: value.result_revision,
            run_id: value.run_id.to_string(),
            artifact_id: value.artifact_id.to_string(),
            needs_review: value.needs_review,
        }
    }
}
