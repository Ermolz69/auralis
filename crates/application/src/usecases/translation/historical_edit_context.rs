use domain::project::ProjectId;
use uuid::Uuid;

pub struct HistoricalTranslationEditContext {
    pub project_id: ProjectId,
    pub translation_id: Uuid,
    pub base_result_id: Uuid,
    pub run_id: Uuid,
    pub base_revision: u32,
    pub expected_head_result_id: Uuid,
    pub head_revision: u32,
    pub expected_link_revision: u64,
}
