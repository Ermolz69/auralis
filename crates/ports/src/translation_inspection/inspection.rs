use super::{TranslationSegment, TranslationSourceFormat};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslationInspection {
    pub format: TranslationSourceFormat,
    pub source_sha256: String,
    pub source_size: u64,
    pub parser_version: u32,
    pub segments: Vec<TranslationSegment>,
}
