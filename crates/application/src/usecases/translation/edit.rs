use super::{
    EditTranslationResultRequest, InspectTranslationSourceRequest, InspectTranslationSourceUseCase,
    linked_result_run,
};
use crate::error::ApplicationError;
use ports::error::PortError;
use ports::translation_inspection::TranslationSourceFormat;
use ports::translation_links::TranslationLinkStore;
use ports::translation_result_edit::{TranslationResultEdit, TranslationResultEditorPort};
use std::sync::Arc;
use uuid::Uuid;

pub struct EditTranslationResultUseCase {
    inspector: Arc<InspectTranslationSourceUseCase>,
    links: Arc<dyn TranslationLinkStore>,
    editor: Arc<dyn TranslationResultEditorPort>,
}

impl EditTranslationResultUseCase {
    pub fn new(
        inspector: Arc<InspectTranslationSourceUseCase>,
        links: Arc<dyn TranslationLinkStore>,
        editor: Arc<dyn TranslationResultEditorPort>,
    ) -> Self {
        Self {
            inspector,
            links,
            editor,
        }
    }

    pub async fn execute(
        &self,
        request: EditTranslationResultRequest,
    ) -> Result<Uuid, ApplicationError> {
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
        let run_id = linked_result_run::resolve(self.links.as_ref(), &link).await?;
        let intent = self.links.run_intent(run_id).await?.ok_or_else(|| {
            ApplicationError::InvalidOperation {
                message: "Active translation has no frozen run intent".into(),
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
        self.editor
            .commit_edit(
                &source_path,
                &link,
                &intent,
                TranslationResultEdit {
                    base_result_id: request.base_result_id,
                    result_id: request.result_id,
                    segment_id: request.segment_id,
                    lines: request.lines,
                },
            )
            .await
            .map_err(Into::into)
    }
}
