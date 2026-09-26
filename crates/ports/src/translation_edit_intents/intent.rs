use chrono::{DateTime, Utc};
use domain::project::ProjectId;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TranslationEditIntent {
    pub project_id: ProjectId,
    pub translation_id: Uuid,
    pub result_id: Uuid,
    pub base_result_id: Uuid,
    pub run_id: Uuid,
    pub expected_head_result_id: Uuid,
    pub expected_link_revision: u64,
    pub request_sha256: String,
    pub created_at: DateTime<Utc>,
}
