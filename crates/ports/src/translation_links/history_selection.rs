use chrono::{DateTime, Utc};
use domain::project::ProjectId;
use uuid::Uuid;

pub struct SelectTranslationHistoryResult {
    pub project_id: ProjectId,
    pub translation_id: Uuid,
    pub result_id: Uuid,
    pub expected_link_revision: u64,
    pub updated_at: DateTime<Utc>,
}
