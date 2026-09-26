use super::super::{
    InspectTranslationSourceRequest, InspectTranslationSourceUseCase, PublishTranslationRequest,
    linked_result_run,
};
use crate::error::ApplicationError;

use ports::error::PortError;
use ports::storage::ArtifactStore;
use ports::translation_inspection::TranslationSourceFormat;
use ports::translation_links::{
    TranslationLinkStore, TranslationPublicationRecord, TranslationPublicationState,
};
use ports::translation_result::TranslationResultPort;
use std::sync::Arc;

#[derive(Clone)]
pub struct PublishTranslationResultUseCase {
    intents: Option<Arc<dyn ports::translation_edit_intents::TranslationEditIntentStore>>,
    pub(super) inspector: Arc<InspectTranslationSourceUseCase>,
    pub(super) links: Arc<dyn TranslationLinkStore>,
    pub(super) results: Arc<dyn TranslationResultPort>,
    pub(super) artifacts: Arc<dyn ArtifactStore>,
}

impl PublishTranslationResultUseCase {
    pub fn new(
        inspector: Arc<InspectTranslationSourceUseCase>,
        links: Arc<dyn TranslationLinkStore>,
        results: Arc<dyn TranslationResultPort>,
        artifacts: Arc<dyn ArtifactStore>,
    ) -> Self {
        Self {
            intents: None,
            inspector,
            links,
            results,
            artifacts,
        }
    }

    pub fn with_edit_intents(
        mut self,
        intents: Arc<dyn ports::translation_edit_intents::TranslationEditIntentStore>,
    ) -> Self {
        self.intents = Some(intents);
        self
    }

    pub async fn execute(
        &self,
        request: PublishTranslationRequest,
    ) -> Result<TranslationPublicationRecord, ApplicationError> {
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
        let edit = if let Some(intents) = &self.intents {
            intents.get(request.result_id).await?
        } else {
            None
        };
        if edit.as_ref().is_some_and(|edit| {
            edit.project_id != request.project_id || edit.translation_id != request.translation_id
        }) {
            return Err(PortError::Conflict {
                resource: "historical translation publication".into(),
                message: "Edit intent belongs to another project or translation".into(),
            }
            .into());
        }
        let run_id = if let Some(edit) = &edit {
            edit.run_id
        } else {
            linked_result_run::resolve(self.links.as_ref(), &link).await?
        };
        let existing = self
            .links
            .get_publication(request.translation_id, request.result_id)
            .await?;
        if let Some(existing) = &existing {
            if existing.source_sha256 != link.source_sha256
                || (run_id != existing.run_id && link.selected_result_id != Some(request.result_id))
                || existing.state == TranslationPublicationState::Failed
            {
                return Err(ApplicationError::InvalidOperation {
                    message: "Stored publication is failed or no longer matches the project link"
                        .into(),
                });
            }
            if edit.is_none() {
                return Ok(existing.clone());
            }
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
                project_id: request.project_id.clone(),
                source_artifact_id: link.source_artifact_id.clone(),
                format,
            })
            .await?;
        if inspection.source_sha256 != link.source_sha256 {
            return Err(ApplicationError::InvalidOperation {
                message: "Original source differs from translation link".into(),
            });
        }
        let result =
            if let Some(edit) = &edit {
                let frozen = self.links.run_intent(edit.run_id).await?.ok_or_else(|| {
                    PortError::NotFound {
                        resource: "historical frozen run".into(),
                    }
                })?;
                let result = self
                    .results
                    .verified_frozen_result(&source_path, &link, &frozen, request.result_id)
                    .await?;
                super::manual::verify(edit, &result)?;
                result
            } else {
                self.results
                    .verified_result(&source_path, &link, run_id, request.result_id)
                    .await?
            };
        if result.result_id != request.result_id || result.run_id != run_id {
            return Err(ApplicationError::InvalidOperation {
                message: "Verified result does not match the active run".into(),
            });
        }
        if let Some(existing) = existing {
            if existing.run_id != result.run_id
                || existing.result_revision != result.result_revision
                || existing.needs_review != result.needs_review
                || edit.as_ref().is_some_and(|edit| {
                    existing.expected_link_revision != edit.expected_link_revision
                })
            {
                return Err(PortError::Conflict {
                    resource: "historical translation publication".into(),
                    message: "Stored publication differs from verified manual result metadata"
                        .into(),
                }
                .into());
            }
            return Ok(existing);
        }
        let expected_link_revision = edit.map_or(link.revision, |edit| edit.expected_link_revision);
        self.stage(request, link, result, expected_link_revision)
            .await
    }
}
