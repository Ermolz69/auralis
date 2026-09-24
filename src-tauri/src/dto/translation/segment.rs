use ports::translation_inspection::TranslationSegment;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationSegmentDto {
    pub id: u32,
    pub ordinal: u32,
    pub cue_label: Option<String>,
    pub start_ms: u64,
    pub end_ms: u64,
    pub source_lines: Vec<String>,
    pub text_ranges: Vec<[u64; 2]>,
}

impl From<TranslationSegment> for TranslationSegmentDto {
    fn from(value: TranslationSegment) -> Self {
        Self {
            id: value.id,
            ordinal: value.ordinal,
            cue_label: value.cue_label,
            start_ms: value.start_ms,
            end_ms: value.end_ms,
            source_lines: value.source_lines,
            text_ranges: value
                .text_ranges
                .into_iter()
                .map(|range| [range.start, range.end])
                .collect(),
        }
    }
}
