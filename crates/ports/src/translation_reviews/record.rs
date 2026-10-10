use chrono::{DateTime, Utc};
use domain::project::ProjectId;
use uuid::Uuid;

pub const MAX_REVIEWER_LABEL_LENGTH: usize = 120;
pub const MAX_REVIEW_NOTE_LENGTH: usize = 2_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReviewDecisionKind {
    Approved,
    Withdrawn,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslationReviewRecord {
    pub event_id: Uuid,
    pub project_id: ProjectId,
    pub translation_id: Uuid,
    pub result_id: Uuid,
    pub result_sha256: String,
    pub decision: ReviewDecisionKind,
    pub reviewer_label: String,
    pub note: String,
    pub created_at: DateTime<Utc>,
}

pub struct RecordTranslationReview {
    pub record: TranslationReviewRecord,
    pub expected_link_revision: u64,
}
