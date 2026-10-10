use super::{RecordTranslationReview, TranslationReviewRecord};
use crate::error::PortError;
use async_trait::async_trait;
use domain::project::ProjectId;
use uuid::Uuid;

#[async_trait]
pub trait TranslationReviewStore: Send + Sync {
    async fn selected_approval_matches(
        &self,
        project_id: &ProjectId,
        translation_id: Uuid,
        result_id: Uuid,
        result_sha256: &str,
        expected_link_revision: u64,
    ) -> Result<bool, PortError>;

    async fn latest(
        &self,
        project_id: &ProjectId,
        translation_id: Uuid,
        result_id: Uuid,
    ) -> Result<Option<TranslationReviewRecord>, PortError>;

    async fn record(
        &self,
        request: RecordTranslationReview,
    ) -> Result<TranslationReviewRecord, PortError>;
}
