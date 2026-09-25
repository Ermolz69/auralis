use super::TranslationRunRequest;
use crate::error::ApplicationError;
use ports::error::PortError;
use ports::translation_links::{TranslationLink, TranslationLinkStore};
use ports::translation_run_control::{TranslationRunControlPort, TranslationRunSnapshot};
use std::sync::Arc;

#[derive(Clone)]
pub struct TranslationRunUseCase {
    links: Arc<dyn TranslationLinkStore>,
    control: Arc<dyn TranslationRunControlPort>,
}

impl TranslationRunUseCase {
    pub fn new(
        links: Arc<dyn TranslationLinkStore>,
        control: Arc<dyn TranslationRunControlPort>,
    ) -> Self {
        Self { links, control }
    }

    pub async fn status(
        &self,
        request: TranslationRunRequest,
    ) -> Result<TranslationRunSnapshot, ApplicationError> {
        let link = self.project_link(&request).await?;
        self.control.snapshot(&link).await.map_err(Into::into)
    }

    pub async fn pause(&self, request: TranslationRunRequest) -> Result<(), ApplicationError> {
        let link = self.project_link(&request).await?;
        self.control.request_pause(&link).await.map_err(Into::into)
    }

    async fn project_link(
        &self,
        request: &TranslationRunRequest,
    ) -> Result<TranslationLink, ApplicationError> {
        let link = self
            .links
            .get(request.translation_id)
            .await?
            .ok_or_else(|| PortError::NotFound {
                resource: "translation link".into(),
            })?;
        if link.project_id != request.project_id {
            return Err(ApplicationError::InvalidOperation {
                message: "Translation belongs to another project".into(),
            });
        }
        Ok(link)
    }
}
