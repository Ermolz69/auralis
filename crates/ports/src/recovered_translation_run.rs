use crate::translation_run_control::TranslationRunPhase;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecoveredTranslationRun {
    pub run_id: Uuid,
    pub phase: TranslationRunPhase,
    pub saved_blocks: usize,
    pub total_blocks: usize,
}
