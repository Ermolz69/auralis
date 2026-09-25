use super::{PublishTranslationRequest, PublishTranslationResultUseCase, TranslationReconcileItem};
use crate::error::ApplicationError;
use ports::translation_links::{TranslationLink, TranslationLinkStore};
use ports::translation_run_control::{TranslationRunControlPort, TranslationRunPhase};
use std::sync::Arc;

pub struct RecoverTranslationPublicationsUseCase {
    links: Arc<dyn TranslationLinkStore>,
    control: Arc<dyn TranslationRunControlPort>,
    publish: PublishTranslationResultUseCase,
}

impl RecoverTranslationPublicationsUseCase {
    pub fn new(
        links: Arc<dyn TranslationLinkStore>,
        control: Arc<dyn TranslationRunControlPort>,
        publish: PublishTranslationResultUseCase,
    ) -> Self {
        Self {
            links,
            control,
            publish,
        }
    }

    pub async fn execute(&self) -> Result<Vec<TranslationReconcileItem>, ApplicationError> {
        let mut outcomes = Vec::new();
        for link in self.links.list_active().await? {
            outcomes.push(TranslationReconcileItem {
                translation_id: link.translation_id,
                outcome: self.recover(link).await,
            });
        }
        Ok(outcomes)
    }

    async fn recover(&self, link: TranslationLink) -> Result<(), ApplicationError> {
        let run_id = link
            .active_run_id
            .ok_or_else(|| ApplicationError::InvalidOperation {
                message: "Active translation has no run ID".into(),
            })?;
        let snapshot = self.control.snapshot(&link).await?;
        if snapshot.run_id != run_id {
            return Err(ApplicationError::InvalidOperation {
                message: "Translate run differs from the active project run".into(),
            });
        }
        if snapshot.phase != TranslationRunPhase::Validated {
            return Ok(());
        }
        let result_id = snapshot
            .result_id
            .ok_or_else(|| ApplicationError::InvalidOperation {
                message: "Validated translation has no result ID".into(),
            })?;
        if self
            .links
            .has_publication_for_run(link.translation_id, run_id)
            .await?
        {
            return Ok(());
        }
        self.publish
            .execute(PublishTranslationRequest {
                project_id: link.project_id,
                translation_id: link.translation_id,
                result_id,
            })
            .await?;
        Ok(())
    }
}
