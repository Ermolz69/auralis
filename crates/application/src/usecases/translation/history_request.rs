use domain::project::ProjectId;
use uuid::Uuid;

pub struct ListTranslationHistoryRequest {
    pub project_id: ProjectId,
    pub translation_id: Uuid,
    pub before_result_id: Option<Uuid>,
    pub limit: u32,
}
