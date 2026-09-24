use super::TranslationRunPhase;
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslationRunSnapshot {
    pub run_id: Uuid,
    pub phase: TranslationRunPhase,
    pub pause_requested: bool,
    pub saved_blocks: usize,
    pub total_blocks: usize,
    pub result_id: Option<Uuid>,
    pub needs_review: Option<bool>,
}
