use domain::project::ProjectId;
use uuid::Uuid;

pub struct EditTranslationResultRequest {
    pub project_id: ProjectId,
    pub translation_id: Uuid,
    pub base_result_id: Uuid,
    pub result_id: Uuid,
    pub segment_id: u32,
    pub lines: Vec<String>,
}
