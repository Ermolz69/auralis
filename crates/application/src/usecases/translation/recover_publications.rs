use super::{
    PublishTranslationRequest, PublishTranslationResultUseCase, TranslationReconcileItem,
    linked_result_run,
};
use crate::error::ApplicationError;
use ports::translation_links::{
    TranslationLink, TranslationLinkStore, TranslationPublicationState,
};
use ports::translation_run_control::{TranslationRunControlPort, TranslationRunPhase};
use std::sync::Arc;

pub struct RecoverTranslationPublicationsUseCase {
    intents: Option<Arc<dyn ports::translation_edit_intents::TranslationEditIntentStore>>,
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
            intents: None,
            links,
            control,
            publish,
        }
    }

    pub fn with_edit_intents(
        mut self,
        intents: Arc<dyn ports::translation_edit_intents::TranslationEditIntentStore>,
    ) -> Self {
        self.intents = Some(intents);
        self
    }

    pub async fn execute(&self) -> Result<Vec<TranslationReconcileItem>, ApplicationError> {
        let mut outcomes = Vec::new();
        if let Some(intents) = &self.intents {
            const RECOVERY_PAGE_SIZE: u32 = 100;
            let mut cursor = None;
            loop {
                let page = intents.list_unpublished(cursor, RECOVERY_PAGE_SIZE).await?;
                if page.is_empty() {
                    break;
                }
                for edit in page {
                    cursor = Some(edit.result_id);
                    outcomes.push(TranslationReconcileItem {
                        translation_id: edit.translation_id,
                        outcome: self
                            .publish
                            .execute(PublishTranslationRequest {
                                project_id: edit.project_id,
                                translation_id: edit.translation_id,
                                result_id: edit.result_id,
                            })
                            .await
                            .map(|_| ()),
                    });
                }
            }
        }
        for link in self.links.list_recoverable().await? {
            outcomes.push(TranslationReconcileItem {
                translation_id: link.translation_id,
                outcome: self.recover(link).await,
            });
        }
        Ok(outcomes)
    }

    async fn recover(&self, link: TranslationLink) -> Result<(), ApplicationError> {
        let run_id = linked_result_run::resolve(self.links.as_ref(), &link).await?;
        let snapshot = self.control.snapshot_for_run(&link, run_id).await?;
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
        if let Some(existing) = self
            .links
            .get_publication(link.translation_id, result_id)
            .await?
        {
            if existing.run_id != run_id || existing.source_sha256 != link.source_sha256 {
                return Err(ApplicationError::InvalidOperation {
                    message: "Latest publication differs from the project source or run".into(),
                });
            }
            if existing.state == TranslationPublicationState::Failed {
                return Err(ApplicationError::InvalidOperation {
                    message: "Latest translation publication failed and needs attention".into(),
                });
            }
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
