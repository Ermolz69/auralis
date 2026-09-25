use domain::project::ProjectId;
use uuid::Uuid;

#[derive(Clone)]
pub struct TranslationRunRequest {
    pub project_id: ProjectId,
    pub translation_id: Uuid,
}
