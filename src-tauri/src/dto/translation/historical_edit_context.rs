use application::usecases::translation::HistoricalTranslationEditContext;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalTranslationEditContextDto {
    pub project_id: String,
    pub translation_id: String,
    pub base_result_id: String,
    pub run_id: String,
    pub base_revision: u32,
    pub expected_head_result_id: String,
    pub head_revision: u32,
    pub expected_link_revision: u64,
}

impl From<HistoricalTranslationEditContext> for HistoricalTranslationEditContextDto {
    fn from(value: HistoricalTranslationEditContext) -> Self {
        Self {
            project_id: value.project_id.to_string(),
            translation_id: value.translation_id.to_string(),
            base_result_id: value.base_result_id.to_string(),
            run_id: value.run_id.to_string(),
            base_revision: value.base_revision,
            expected_head_result_id: value.expected_head_result_id.to_string(),
            head_revision: value.head_revision,
            expected_link_revision: value.expected_link_revision,
        }
    }
}
