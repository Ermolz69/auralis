use std::ops::Range;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslationSegment {
    pub id: u32,
    pub ordinal: u32,
    pub cue_label: Option<String>,
    pub start_ms: u64,
    pub end_ms: u64,
    pub source_lines: Vec<String>,
    pub text_ranges: Vec<Range<u64>>,
}
