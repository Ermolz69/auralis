use domain::project::ProjectId;
use ports::translation_reviews::ReviewDecisionKind;
use uuid::Uuid;

pub struct GetTranslationReviewRequest {
    pub project_id: ProjectId,
    pub translation_id: Uuid,
    pub result_id: Uuid,
}

pub struct RecordTranslationReviewRequest {
    pub project_id: ProjectId,
    pub translation_id: Uuid,
    pub result_id: Uuid,
    pub expected_result_sha256: String,
    pub expected_link_revision: u64,
    pub decision: ReviewDecisionKind,
    pub reviewer_label: String,
    pub note: String,
}
