use ports::translation_links::TranslationLink;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationLinkDto {
    pub translation_id: String,
    pub project_id: String,
    pub source_artifact_id: String,
    pub source_sha256: String,
    pub source_format: String,
    pub source_language: String,
    pub target_language: String,
    pub active_run_id: Option<String>,
    pub selected_result_id: Option<String>,
    pub output_artifact_id: Option<String>,
    pub revision: u64,
    pub created_at: String,
    pub updated_at: String,
}

impl From<TranslationLink> for TranslationLinkDto {
    fn from(value: TranslationLink) -> Self {
        Self {
            translation_id: value.translation_id.to_string(),
            project_id: value.project_id.to_string(),
            source_artifact_id: value.source_artifact_id.to_string(),
            source_sha256: value.source_sha256,
            source_format: value.source_format,
            source_language: value.source_language,
            target_language: value.target_language,
            active_run_id: value.active_run_id.map(|id| id.to_string()),
            selected_result_id: value.selected_result_id.map(|id| id.to_string()),
            output_artifact_id: value.output_artifact_id.map(|id| id.to_string()),
            revision: value.revision,
            created_at: value.created_at.to_rfc3339(),
            updated_at: value.updated_at.to_rfc3339(),
        }
    }
}
