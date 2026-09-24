use domain::project::ProjectId;
use uuid::Uuid;

pub struct GetTranslationComparisonRequest {
    pub project_id: ProjectId,
    pub translation_id: Uuid,
    pub result_id: Uuid,
    pub offset: u32,
    pub limit: u32,
}
