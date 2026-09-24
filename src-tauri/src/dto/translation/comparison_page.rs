use super::ComparedTranslationSegmentDto;
use ports::translation_result::TranslationComparisonPage;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationComparisonPageDto {
    pub result_id: String,
    pub run_id: String,
    pub needs_review: bool,
    pub total_segments: u32,
    pub offset: u32,
    pub segments: Vec<ComparedTranslationSegmentDto>,
}

impl From<TranslationComparisonPage> for TranslationComparisonPageDto {
    fn from(value: TranslationComparisonPage) -> Self {
        Self {
            result_id: value.result_id.to_string(),
            run_id: value.run_id.to_string(),
            needs_review: value.needs_review,
            total_segments: value.total_segments,
            offset: value.offset,
            segments: value.segments.into_iter().map(Into::into).collect(),
        }
    }
}
