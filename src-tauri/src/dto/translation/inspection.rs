use super::{TranslationFormatDto, TranslationSegmentDto};
use ports::translation_inspection::TranslationInspection;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationInspectionDto {
    pub format: TranslationFormatDto,
    pub source_sha256: String,
    pub source_size: u64,
    pub parser_version: u32,
    pub segments: Vec<TranslationSegmentDto>,
}

impl From<TranslationInspection> for TranslationInspectionDto {
    fn from(value: TranslationInspection) -> Self {
        Self {
            format: value.format.into(),
            source_sha256: value.source_sha256,
            source_size: value.source_size,
            parser_version: value.parser_version,
            segments: value.segments.into_iter().map(Into::into).collect(),
        }
    }
}
