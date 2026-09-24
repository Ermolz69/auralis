use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommittedTranslationProgress {
    pub run_id: Uuid,
    pub saved_blocks: usize,
    pub total_blocks: usize,
}
