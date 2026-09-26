use domain::project::ProjectId;
use uuid::Uuid;

pub struct GetHistoricalTranslationEditContextRequest {
    pub project_id: ProjectId,
    pub translation_id: Uuid,
    pub base_result_id: Uuid,
}
