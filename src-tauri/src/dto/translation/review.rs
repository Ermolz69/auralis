use application::usecases::translation::TranslationReviewView;
use ports::translation_reviews::{ReviewDecisionKind, TranslationReviewRecord};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationReviewDecisionDto {
    Approved,
    Withdrawn,
}

impl From<TranslationReviewDecisionDto> for ReviewDecisionKind {
    fn from(value: TranslationReviewDecisionDto) -> Self {
        match value {
            TranslationReviewDecisionDto::Approved => Self::Approved,
            TranslationReviewDecisionDto::Withdrawn => Self::Withdrawn,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationReviewRecordDto {
    pub event_id: String,
    pub decision: &'static str,
    pub reviewer_label: String,
    pub note: String,
    pub created_at: String,
}

impl From<TranslationReviewRecord> for TranslationReviewRecordDto {
    fn from(value: TranslationReviewRecord) -> Self {
        Self {
            event_id: value.event_id.to_string(),
            decision: match value.decision {
                ReviewDecisionKind::Approved => "approved",
                ReviewDecisionKind::Withdrawn => "withdrawn",
            },
            reviewer_label: value.reviewer_label,
            note: value.note,
            created_at: value.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationReviewViewDto {
    pub result_id: String,
    pub result_sha256: String,
    pub link_revision: u64,
    pub needs_review: bool,
    pub selected: bool,
    pub latest: Option<TranslationReviewRecordDto>,
}

impl From<TranslationReviewView> for TranslationReviewViewDto {
    fn from(value: TranslationReviewView) -> Self {
        Self {
            result_id: value.result_id.to_string(),
            result_sha256: value.result_sha256,
            link_revision: value.link_revision,
            needs_review: value.needs_review,
            selected: value.selected,
            latest: value.latest.map(Into::into),
        }
    }
}
