use ports::translation_result::ComparedTranslationSegment;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparedTranslationSegmentDto {
    pub id: u32,
    pub ordinal: u32,
    pub cue_label: Option<String>,
    pub start_ms: u64,
    pub end_ms: u64,
    pub source_lines: Vec<String>,
    pub translated_lines: Vec<String>,
}

impl From<ComparedTranslationSegment> for ComparedTranslationSegmentDto {
    fn from(value: ComparedTranslationSegment) -> Self {
        Self {
            id: value.id,
            ordinal: value.ordinal,
            cue_label: value.cue_label,
            start_ms: value.start_ms,
            end_ms: value.end_ms,
            source_lines: value.source_lines,
            translated_lines: value.translated_lines,
        }
    }
}
