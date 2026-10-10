use super::{
    GetTranslationReviewRequest, RecordTranslationReviewRequest, TranslationReviewView,
    VerifiedTranslationHistory, VerifyTranslationHistoryUseCase,
};
use crate::error::ApplicationError;
use ports::error::PortError;
use ports::translation_reviews::{
    RecordTranslationReview, TranslationReviewRecord, TranslationReviewStore,
};
use std::sync::Arc;
use uuid::Uuid;

pub struct TranslationReviewUseCase {
    verifier: Arc<VerifyTranslationHistoryUseCase>,
    reviews: Arc<dyn TranslationReviewStore>,
}

impl TranslationReviewUseCase {
    pub fn new(
        verifier: Arc<VerifyTranslationHistoryUseCase>,
        reviews: Arc<dyn TranslationReviewStore>,
    ) -> Self {
        Self { verifier, reviews }
    }

    pub async fn get(
        &self,
        request: GetTranslationReviewRequest,
    ) -> Result<TranslationReviewView, ApplicationError> {
        let verified = self
            .verifier
            .execute(
                request.project_id,
                request.translation_id,
                request.result_id,
            )
            .await?;
        let latest = self
            .reviews
            .latest(
                &verified.link.project_id,
                request.translation_id,
                request.result_id,
            )
            .await?;
        ensure_digest(&latest, &verified.result.output_sha256)?;
        Ok(view(verified, latest))
    }

    pub async fn record(
        &self,
        request: RecordTranslationReviewRequest,
    ) -> Result<TranslationReviewView, ApplicationError> {
        let verified = self
            .verifier
            .execute(
                request.project_id.clone(),
                request.translation_id,
                request.result_id,
            )
            .await?;
        if verified.result.output_sha256 != request.expected_result_sha256
            || verified.link.revision != request.expected_link_revision
        {
            return Err(conflict(
                "result digest or project link changed before review",
            ));
        }
        let latest = self
            .reviews
            .record(RecordTranslationReview {
                record: TranslationReviewRecord {
                    event_id: Uuid::new_v4(),
                    project_id: request.project_id,
                    translation_id: request.translation_id,
                    result_id: request.result_id,
                    result_sha256: verified.result.output_sha256.clone(),
                    decision: request.decision,
                    reviewer_label: request.reviewer_label,
                    note: request.note,
                    created_at: domain::chrono::Utc::now(),
                },
                expected_link_revision: request.expected_link_revision,
            })
            .await?;
        Ok(view(verified, Some(latest)))
    }

    pub async fn require_approved_selected(
        &self,
        project_id: domain::project::ProjectId,
        translation_id: Uuid,
    ) -> Result<VerifiedTranslationHistory, ApplicationError> {
        let link = self
            .verifier
            .link_for_project(&project_id, translation_id)
            .await?;
        let result_id = link
            .selected_result_id
            .ok_or_else(|| conflict("translation has no selected result for speech"))?;
        let verified = self
            .verifier
            .execute(project_id, translation_id, result_id)
            .await?;
        if !self
            .reviews
            .selected_approval_matches(
                &verified.link.project_id,
                translation_id,
                result_id,
                &verified.result.output_sha256,
                verified.link.revision,
            )
            .await?
        {
            return Err(conflict("selected translation is not approved for speech"));
        }
        Ok(verified)
    }
}

fn view(
    verified: VerifiedTranslationHistory,
    latest: Option<TranslationReviewRecord>,
) -> TranslationReviewView {
    TranslationReviewView {
        result_id: verified.result.result_id,
        result_sha256: verified.result.output_sha256,
        link_revision: verified.link.revision,
        needs_review: verified.result.needs_review,
        selected: verified.link.selected_result_id == Some(verified.result.result_id),
        latest,
    }
}

fn ensure_digest(
    latest: &Option<TranslationReviewRecord>,
    verified_digest: &str,
) -> Result<(), ApplicationError> {
    if latest
        .as_ref()
        .is_some_and(|event| event.result_sha256 != verified_digest)
    {
        return Err(conflict(
            "stored review digest differs from verified output",
        ));
    }
    Ok(())
}

fn conflict(message: &str) -> ApplicationError {
    PortError::Conflict {
        resource: "translation review".into(),
        message: message.into(),
    }
    .into()
}
