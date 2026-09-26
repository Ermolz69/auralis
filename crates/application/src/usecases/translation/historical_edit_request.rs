use domain::project::ProjectId;
use uuid::Uuid;

pub struct EditHistoricalTranslationRequest {
    pub project_id: ProjectId,
    pub translation_id: Uuid,
    pub base_result_id: Uuid,
    pub result_id: Uuid,
    pub expected_head_result_id: Uuid,
    pub expected_link_revision: u64,
    pub segment_id: u32,
    pub lines: Vec<String>,
}
