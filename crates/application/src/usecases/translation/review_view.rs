use ports::translation_reviews::TranslationReviewRecord;
use uuid::Uuid;

pub struct TranslationReviewView {
    pub result_id: Uuid,
    pub result_sha256: String,
    pub link_revision: u64,
    pub needs_review: bool,
    pub selected: bool,
    pub latest: Option<TranslationReviewRecord>,
}
