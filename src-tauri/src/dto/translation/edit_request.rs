use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditTranslationResultDto {
    pub project_id: String,
    pub translation_id: String,
    pub base_result_id: String,
    pub result_id: String,
    pub segment_id: u32,
    pub lines: Vec<String>,
}
