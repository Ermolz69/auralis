use chrono::{DateTime, Utc};
use domain::media::ArtifactId;
use domain::project::ProjectId;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranslationLink {
    pub translation_id: Uuid,
    pub project_id: ProjectId,
    pub source_artifact_id: ArtifactId,
    pub source_sha256: String,
    pub source_format: String,
    pub source_language: String,
    pub target_language: String,
    pub active_run_id: Option<Uuid>,
    pub selected_result_id: Option<Uuid>,
    pub output_artifact_id: Option<ArtifactId>,
    pub revision: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
