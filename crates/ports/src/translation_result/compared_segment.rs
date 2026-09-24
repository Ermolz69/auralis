pub struct ComparedTranslationSegment {
    pub id: u32,
    pub ordinal: u32,
    pub cue_label: Option<String>,
    pub start_ms: u64,
    pub end_ms: u64,
    pub source_lines: Vec<String>,
    pub translated_lines: Vec<String>,
}
