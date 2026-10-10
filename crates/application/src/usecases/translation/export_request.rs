use domain::project::ProjectId;
use std::path::PathBuf;
use uuid::Uuid;

pub struct ExportTranslationResultRequest {
    pub project_id: ProjectId,
    pub translation_id: Uuid,
    pub result_id: Uuid,
    pub expected_result_sha256: String,
    pub destination: PathBuf,
}
