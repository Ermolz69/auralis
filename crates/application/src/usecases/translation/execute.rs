use super::{
    ExecuteTranslationRequest, InspectTranslationSourceRequest, InspectTranslationSourceUseCase,
};
use crate::error::ApplicationError;
use domain::job::JobId;
use ports::committed_translation_progress::CommittedTranslationProgress;
use ports::error::PortError;
use ports::translation_execution::TranslationExecutionPort;
use ports::translation_inspection::TranslationSourceFormat;
use ports::translation_links::TranslationLinkStore;
use ports::translation_run_registration::TranslationRunRegistrationPort;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

pub struct ExecuteTranslationUseCase {
    inspector: Arc<InspectTranslationSourceUseCase>,
    links: Arc<dyn TranslationLinkStore>,
    registrar: Arc<dyn TranslationRunRegistrationPort>,
    executor: Arc<dyn TranslationExecutionPort>,
}

impl ExecuteTranslationUseCase {
    pub fn new(
        inspector: Arc<InspectTranslationSourceUseCase>,
        links: Arc<dyn TranslationLinkStore>,
        registrar: Arc<dyn TranslationRunRegistrationPort>,
        executor: Arc<dyn TranslationExecutionPort>,
    ) -> Self {
        Self {
            inspector,
            links,
            registrar,
            executor,
        }
    }

    pub async fn execute(
        &self,
        request: ExecuteTranslationRequest,
    ) -> Result<Uuid, ApplicationError> {
        self.execute_inner(request, None).await
    }

    pub async fn execute_for_host(
        &self,
        request: ExecuteTranslationRequest,
        job_id: &JobId,
        progress: UnboundedSender<CommittedTranslationProgress>,
    ) -> Result<Uuid, ApplicationError> {
        self.execute_inner(request, Some((job_id, progress))).await
    }

    async fn execute_inner(
        &self,
        request: ExecuteTranslationRequest,
        host: Option<(&JobId, UnboundedSender<CommittedTranslationProgress>)>,
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
        let run_id = link
            .active_run_id
            .ok_or_else(|| ApplicationError::InvalidOperation {
                message: "Translation has no active run".into(),
            })?;
        let intent = self.links.run_intent(run_id).await?.ok_or_else(|| {
            ApplicationError::InvalidOperation {
                message: "Active translation has no frozen run intent".into(),
            }
        })?;
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
        self.registrar
            .ensure_run(&source_path, &link, &intent)
            .await?;
        match host {
            Some((job_id, progress)) => self
                .executor
                .execute_for_host(
                    &source_path,
                    &link,
                    &intent,
                    &request.server_url,
                    job_id,
                    progress,
                )
                .await
                .map_err(Into::into),
            None => self
                .executor
                .execute(&source_path, &link, &intent, &request.server_url)
                .await
                .map_err(Into::into),
        }
    }
}
