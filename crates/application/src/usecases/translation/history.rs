use super::ListTranslationHistoryRequest;
use crate::error::ApplicationError;
use ports::translation_links::{TranslationHistoryPage, TranslationLinkStore};
use std::sync::Arc;

pub struct ListTranslationHistoryUseCase {
    links: Arc<dyn TranslationLinkStore>,
}

impl ListTranslationHistoryUseCase {
    pub fn new(links: Arc<dyn TranslationLinkStore>) -> Self {
        Self { links }
    }

    pub async fn execute(
        &self,
        request: ListTranslationHistoryRequest,
    ) -> Result<TranslationHistoryPage, ApplicationError> {
        self.links
            .list_ready_history(
                &request.project_id,
                request.translation_id,
                request.before_result_id,
                request.limit,
            )
            .await
            .map_err(Into::into)
    }
}
