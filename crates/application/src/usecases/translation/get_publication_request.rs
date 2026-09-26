use domain::project::ProjectId;
use uuid::Uuid;

pub struct GetTranslationPublicationRequest {
    pub project_id: ProjectId,
    pub translation_id: Uuid,
    pub result_id: Uuid,
}
