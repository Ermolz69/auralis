use super::ComparedTranslationSegment;
use uuid::Uuid;

pub const MAX_COMPARISON_PAGE_SIZE: u32 = 100;

pub struct TranslationComparisonPage {
    pub result_id: Uuid,
    pub run_id: Uuid,
    pub needs_review: bool,
    pub total_segments: u32,
    pub offset: u32,
    pub segments: Vec<ComparedTranslationSegment>,
}
