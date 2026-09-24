use domain::project::ProjectId;
use uuid::Uuid;

pub struct ExecuteTranslationRequest {
    pub project_id: ProjectId,
    pub translation_id: Uuid,
    pub server_url: String,
}
