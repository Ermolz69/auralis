use super::ComparedTranslationSegment;
use uuid::Uuid;

pub struct VerifiedTranslationResult {
    pub result_id: Uuid,
    pub run_id: Uuid,
    pub output_sha256: String,
    pub output_bytes: Vec<u8>,
    pub needs_review: bool,
    pub segments: Vec<ComparedTranslationSegment>,
}
