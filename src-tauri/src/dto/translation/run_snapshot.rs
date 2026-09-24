use super::run_phase::TranslationRunPhaseDto;
use ports::translation_run_control::TranslationRunSnapshot;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationRunSnapshotDto {
    pub run_id: String,
    pub phase: TranslationRunPhaseDto,
    pub pause_requested: bool,
    pub saved_blocks: usize,
    pub total_blocks: usize,
    pub result_id: Option<String>,
    pub needs_review: Option<bool>,
}

impl From<TranslationRunSnapshot> for TranslationRunSnapshotDto {
    fn from(value: TranslationRunSnapshot) -> Self {
        Self {
            run_id: value.run_id.to_string(),
            phase: value.phase.into(),
            pause_requested: value.pause_requested,
            saved_blocks: value.saved_blocks,
            total_blocks: value.total_blocks,
            result_id: value.result_id.map(|id| id.to_string()),
            needs_review: value.needs_review,
        }
    }
}
