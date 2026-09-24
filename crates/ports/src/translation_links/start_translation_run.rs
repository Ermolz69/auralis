use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct StartTranslationRun {
    pub translation_id: Uuid,
    pub run_id: Uuid,
    pub expected_active_run_id: Option<Uuid>,
    pub expected_link_revision: u64,
    pub updated_at: DateTime<Utc>,
}
