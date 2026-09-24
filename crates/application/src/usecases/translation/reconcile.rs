use super::{
    InspectTranslationSourceRequest, InspectTranslationSourceUseCase, TranslationReconcileItem,
};
use crate::error::ApplicationError;
use ports::translation_inspection::TranslationSourceFormat;
use ports::translation_links::{TranslationLink, TranslationLinkStore};
use ports::translation_run_registration::TranslationRunRegistrationPort;
use std::sync::Arc;

pub struct ReconcileTranslationRunsUseCase {
    inspector: Arc<InspectTranslationSourceUseCase>,
    links: Arc<dyn TranslationLinkStore>,
    registrar: Arc<dyn TranslationRunRegistrationPort>,
}

impl ReconcileTranslationRunsUseCase {
    pub fn new(
        inspector: Arc<InspectTranslationSourceUseCase>,
        links: Arc<dyn TranslationLinkStore>,
        registrar: Arc<dyn TranslationRunRegistrationPort>,
    ) -> Self {
        Self {
            inspector,
            links,
            registrar,
        }
    }

    pub async fn execute(&self) -> Result<Vec<TranslationReconcileItem>, ApplicationError> {
        let mut outcomes = Vec::new();
        for link in self.links.list_active().await? {
            outcomes.push(TranslationReconcileItem {
                translation_id: link.translation_id,
                outcome: self.reconcile(link).await,
            });
        }
        Ok(outcomes)
    }

    async fn reconcile(&self, link: TranslationLink) -> Result<(), ApplicationError> {
        let run_id = link
            .active_run_id
            .ok_or_else(|| ApplicationError::InvalidOperation {
                message: "Active translation link has no run ID".into(),
            })?;
        let intent = self.links.run_intent(run_id).await?.ok_or_else(|| {
            ApplicationError::InvalidOperation {
                message: "Active translation has no frozen run intent".into(),
            }
        })?;
        if intent.translation_id != link.translation_id {
            return Err(ApplicationError::InvalidOperation {
                message: "Frozen run intent belongs to another translation".into(),
            });
        }
        let format = match link.source_format.as_str() {
            "srt" => TranslationSourceFormat::Srt,
            "vtt" => TranslationSourceFormat::Vtt,
            _ => {
                return Err(ApplicationError::InvalidOperation {
                    message: "Active translation has an unsupported source format".into(),
                });
            }
        };
        let (path, report) = self
            .inspector
            .inspect_with_path(InspectTranslationSourceRequest {
                project_id: link.project_id.clone(),
                source_artifact_id: link.source_artifact_id.clone(),
                format,
            })
            .await?;
        if report.source_sha256 != link.source_sha256
            || report.parser_version != intent.parser_version
        {
            return Err(ApplicationError::InvalidOperation {
                message: "Original source or parser differs from frozen run intent".into(),
            });
        }
        self.registrar.ensure_run(&path, &link, &intent).await?;
        Ok(())
    }
}
