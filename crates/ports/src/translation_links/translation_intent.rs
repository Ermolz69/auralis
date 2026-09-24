use chrono::{DateTime, Utc};
use domain::media::ArtifactId;
use domain::project::ProjectId;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct TranslationIntent {
    pub translation_id: Uuid,
    pub project_id: ProjectId,
    pub source_artifact_id: ArtifactId,
    pub source_sha256: String,
    pub source_format: String,
    pub source_language: String,
    pub target_language: String,
    pub run_id: Uuid,
    pub created_at: DateTime<Utc>,
}
