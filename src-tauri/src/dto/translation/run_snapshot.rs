use super::run_phase::TranslationRunPhaseDto;
use application::usecases::translation::TranslationRunStatus;
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
    pub active_host_job_id: Option<String>,
}

impl From<TranslationRunStatus> for TranslationRunSnapshotDto {
    fn from(status: TranslationRunStatus) -> Self {
        let value = status.snapshot;
        Self {
            run_id: value.run_id.to_string(),
            phase: value.phase.into(),
            pause_requested: value.pause_requested,
            saved_blocks: value.saved_blocks,
            total_blocks: value.total_blocks,
            result_id: value.result_id.map(|id| id.to_string()),
            needs_review: value.needs_review,
            active_host_job_id: status.active_host_job_id.map(|id| id.to_string()),
        }
    }
}
