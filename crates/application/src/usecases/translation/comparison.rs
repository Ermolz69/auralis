use super::{
    GetTranslationComparisonRequest, InspectTranslationSourceRequest,
    InspectTranslationSourceUseCase, linked_result_run,
};
use crate::error::ApplicationError;
use ports::error::PortError;
use ports::translation_inspection::TranslationSourceFormat;
use ports::translation_links::{TranslationLinkStore, TranslationPublicationState};
use ports::translation_result::{
    MAX_COMPARISON_PAGE_SIZE, TranslationComparisonPage, TranslationResultPort,
};
use std::sync::Arc;

pub struct GetTranslationComparisonUseCase {
    inspector: Arc<InspectTranslationSourceUseCase>,
    links: Arc<dyn TranslationLinkStore>,
    results: Arc<dyn TranslationResultPort>,
}

impl GetTranslationComparisonUseCase {
    pub fn new(
        inspector: Arc<InspectTranslationSourceUseCase>,
        links: Arc<dyn TranslationLinkStore>,
        results: Arc<dyn TranslationResultPort>,
    ) -> Self {
        Self {
            inspector,
            links,
            results,
        }
    }

    pub async fn execute(
        &self,
        request: GetTranslationComparisonRequest,
    ) -> Result<TranslationComparisonPage, ApplicationError> {
        if !(1..=MAX_COMPARISON_PAGE_SIZE).contains(&request.limit) {
            return Err(ApplicationError::InvalidOperation {
                message: "Translation comparison page size is invalid".into(),
            });
        }
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
        let run_id = match self
            .links
            .get_publication(request.translation_id, request.result_id)
            .await?
        {
            Some(publication)
                if publication.state == TranslationPublicationState::Ready
                    && publication.source_sha256 == link.source_sha256 =>
            {
                publication.run_id
            }
            Some(publication) if publication.state == TranslationPublicationState::Failed => {
                return Err(ApplicationError::InvalidOperation {
                    message: "Failed publication cannot be compared".into(),
                });
            }
            _ => linked_result_run::resolve(self.links.as_ref(), &link).await?,
        };
        let intent = self.links.run_intent(run_id).await?.ok_or_else(|| {
            ApplicationError::InvalidOperation {
                message: "Translation result has no frozen run intent".into(),
            }
        })?;
        if intent.translation_id != request.translation_id {
            return Err(ApplicationError::InvalidOperation {
                message: "Frozen run belongs to another translation".into(),
            });
        }
        let format = match link.source_format.as_str() {
            "srt" => TranslationSourceFormat::Srt,
            "vtt" => TranslationSourceFormat::Vtt,
            _ => {
                return Err(ApplicationError::InvalidOperation {
                    message: "Translation format is unsupported".into(),
                });
            }
        };
        let (source_path, inspection) = self
            .inspector
            .inspect_with_path(InspectTranslationSourceRequest {
                project_id: request.project_id,
                source_artifact_id: link.source_artifact_id.clone(),
                format,
            })
            .await?;
        if inspection.source_sha256 != link.source_sha256
            || inspection.parser_version != intent.parser_version
        {
            return Err(ApplicationError::InvalidOperation {
                message: "Original source or parser differs from frozen run".into(),
            });
        }
        self.results
            .comparison_page(
                &source_path,
                &link,
                run_id,
                request.result_id,
                request.offset,
                request.limit,
            )
            .await
            .map_err(Into::into)
    }
}
