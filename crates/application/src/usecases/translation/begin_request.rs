use chrono::{DateTime, Utc};
use domain::media::ArtifactId;
use domain::project::ProjectId;
use ports::translation_inspection::TranslationSourceFormat;
use uuid::Uuid;

pub struct BeginTranslationRequest {
    pub project_id: ProjectId,
    pub source_artifact_id: ArtifactId,
    pub format: TranslationSourceFormat,
    pub source_language: String,
    pub translation_id: Uuid,
    pub run_id: Uuid,
    pub profile_manifest_json: String,
    pub created_at: DateTime<Utc>,
}
