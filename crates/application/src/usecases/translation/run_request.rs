use domain::project::ProjectId;
use uuid::Uuid;

pub struct TranslationRunRequest {
    pub project_id: ProjectId,
    pub translation_id: Uuid,
}
