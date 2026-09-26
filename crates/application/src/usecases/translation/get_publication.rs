use super::GetTranslationPublicationRequest;
use crate::error::ApplicationError;
use ports::translation_links::{TranslationLinkStore, TranslationPublicationView};
use std::sync::Arc;

pub struct GetTranslationPublicationUseCase {
    links: Arc<dyn TranslationLinkStore>,
}

impl GetTranslationPublicationUseCase {
    pub fn new(links: Arc<dyn TranslationLinkStore>) -> Self {
        Self { links }
    }
    pub async fn execute(
        &self,
        request: GetTranslationPublicationRequest,
    ) -> Result<TranslationPublicationView, ApplicationError> {
        self.links
            .publication_view(
                &request.project_id,
                request.translation_id,
                request.result_id,
            )
            .await
            .map_err(Into::into)
    }
}
