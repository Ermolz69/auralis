use domain::project::ProjectId;
use uuid::Uuid;

pub struct SelectTranslationResultRequest {
    pub project_id: ProjectId,
    pub translation_id: Uuid,
    pub result_id: Uuid,
    pub expected_link_revision: u64,
}
