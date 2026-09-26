use super::{InspectTranslationSourceUseCase, SelectTranslationResultRequest};
use crate::error::ApplicationError;
use ports::artifact_index::ArtifactIndex;
use ports::storage::ArtifactStore;
use ports::translation_links::{
    SelectTranslationHistoryResult, TranslationLink, TranslationLinkStore,
};
use ports::translation_result::TranslationResultPort;
use std::sync::Arc;

pub struct SelectTranslationResultUseCase {
    verifier: super::VerifyTranslationHistoryUseCase,
    links: Arc<dyn TranslationLinkStore>,
}
impl SelectTranslationResultUseCase {
    pub fn new(
        inspector: Arc<InspectTranslationSourceUseCase>,
        links: Arc<dyn TranslationLinkStore>,
        results: Arc<dyn TranslationResultPort>,
        artifacts: Arc<dyn ArtifactIndex>,
        files: Arc<dyn ArtifactStore>,
    ) -> Self {
        Self {
            verifier: super::VerifyTranslationHistoryUseCase::new(
                inspector,
                links.clone(),
                results,
                artifacts,
                files,
            ),
            links,
        }
    }
    pub async fn execute(
        &self,
        request: SelectTranslationResultRequest,
    ) -> Result<TranslationLink, ApplicationError> {
        self.verifier
            .execute(
                request.project_id.clone(),
                request.translation_id,
                request.result_id,
            )
            .await?;
        self.links
            .select_historical_result(SelectTranslationHistoryResult {
                project_id: request.project_id,
                translation_id: request.translation_id,
                result_id: request.result_id,
                expected_link_revision: request.expected_link_revision,
                updated_at: domain::chrono::Utc::now(),
            })
            .await
            .map_err(Into::into)
    }
}
