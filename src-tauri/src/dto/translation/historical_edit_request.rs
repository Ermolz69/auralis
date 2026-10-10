use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditHistoricalTranslationResultDto {
    pub project_id: String,
    pub translation_id: String,
    pub base_result_id: String,
    pub result_id: String,
    pub expected_head_result_id: String,
    pub expected_link_revision: u64,
    pub segment_id: u32,
    pub lines: Vec<String>,
}
