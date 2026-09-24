use super::{
    BeginTranslationRequest, InspectTranslationSourceRequest, InspectTranslationSourceUseCase,
};
use crate::error::ApplicationError;
use ports::translation_inspection::TranslationSourceFormat;
use ports::translation_links::{TranslationIntent, TranslationLink, TranslationLinkStore};
use ports::translation_run_registration::TranslationRunRegistrationPort;
use std::sync::Arc;

pub struct BeginTranslationUseCase {
    inspector: Arc<InspectTranslationSourceUseCase>,
    links: Arc<dyn TranslationLinkStore>,
    registrar: Arc<dyn TranslationRunRegistrationPort>,
}

impl BeginTranslationUseCase {
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

    pub async fn execute(
        &self,
        request: BeginTranslationRequest,
    ) -> Result<TranslationLink, ApplicationError> {
        if !matches!(request.source_language.as_str(), "zh" | "ja") {
            return Err(ApplicationError::InvalidOperation {
                message: "Source language must be Chinese or Japanese".into(),
            });
        }
        let (source_path, inspection) = self
            .inspector
            .inspect_with_path(InspectTranslationSourceRequest {
                project_id: request.project_id.clone(),
                source_artifact_id: request.source_artifact_id.clone(),
                format: request.format,
            })
            .await?;
        let created_at = self
            .links
            .run_intent(request.run_id)
            .await?
            .map_or(request.created_at, |existing| existing.created_at);
        let run = self.registrar.prepare_intent(
            request.format,
            request.translation_id,
            request.run_id,
            request.profile_manifest_json,
            created_at,
        )?;
        if run.parser_version != inspection.parser_version {
            return Err(ApplicationError::InvalidOperation {
                message: "Inspector and run parser versions differ".into(),
            });
        }
        let source_format = match request.format {
            TranslationSourceFormat::Srt => "srt",
            TranslationSourceFormat::Vtt => "vtt",
        };
        let link = self
            .links
            .begin_with_run_intent(
                TranslationIntent {
                    translation_id: request.translation_id,
                    project_id: request.project_id,
                    source_artifact_id: request.source_artifact_id,
                    source_sha256: inspection.source_sha256,
                    source_format: source_format.into(),
                    source_language: request.source_language,
                    target_language: "ru".into(),
                    run_id: request.run_id,
                    created_at: request.created_at,
                },
                run.clone(),
            )
            .await?;
        self.registrar.ensure_run(&source_path, &link, &run).await?;
        Ok(link)
    }
}
