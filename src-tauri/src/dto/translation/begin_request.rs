use super::{TranslationFormatDto, TranslationSourceLanguageDto};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BeginTranslationRunDto {
    pub project_id: String,
    pub source_artifact_id: String,
    pub format: TranslationFormatDto,
    pub source_language: TranslationSourceLanguageDto,
    pub translation_id: String,
    pub run_id: String,
}
