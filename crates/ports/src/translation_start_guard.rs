use crate::translation_run_control::TranslationRunPhase;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TranslationStartGuard {
    pub run_id: Uuid,
    pub phase: TranslationRunPhase,
    pub control_revision: u64,
}
