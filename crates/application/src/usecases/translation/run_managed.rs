use super::{
    ExecuteTranslationRequest, PublishTranslationRequest, PublishTranslationResultUseCase,
    RunTranslationHostJobUseCase, StartTranslationJobUseCase, TranslationRunRequest,
};
use crate::error::ApplicationError;
use ports::error::PortError;
use ports::translation_links::{TranslationLinkStore, TranslationPublicationRecord};
use ports::translation_model_runtime::TranslationModelRuntimePort;
use ports::translation_run_control::{TranslationRunControlPort, TranslationRunPhase};
use std::sync::Arc;

pub struct RunManagedTranslationUseCase {
    links: Arc<dyn TranslationLinkStore>,
    runtime: Arc<dyn TranslationModelRuntimePort>,
    control: Arc<dyn TranslationRunControlPort>,
    start_job: StartTranslationJobUseCase,
    run_job: RunTranslationHostJobUseCase,
    publish: PublishTranslationResultUseCase,
}

impl RunManagedTranslationUseCase {
    pub fn new(
        links: Arc<dyn TranslationLinkStore>,
        runtime: Arc<dyn TranslationModelRuntimePort>,
        control: Arc<dyn TranslationRunControlPort>,
        start_job: StartTranslationJobUseCase,
        run_job: RunTranslationHostJobUseCase,
        publish: PublishTranslationResultUseCase,
    ) -> Self {
        Self {
            links,
            runtime,
            control,
            start_job,
            run_job,
            publish,
        }
    }

    pub async fn execute(
        &self,
        request: TranslationRunRequest,
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
        if intent.translation_id != request.translation_id {
            return Err(ApplicationError::InvalidOperation {
                message: "Frozen run belongs to another translation".into(),
            });
        }
        let snapshot = self.control.snapshot(&link).await?;
        if snapshot.run_id != run_id {
            return Err(ApplicationError::InvalidOperation {
                message: "Translate run differs from the active project run".into(),
            });
        }
        if matches!(
            snapshot.phase,
            TranslationRunPhase::Validated | TranslationRunPhase::Running
        ) {
            return Err(ApplicationError::InvalidOperation {
                message: "Translation run cannot start a new managed attempt".into(),
            });
        }
        let lease = self.runtime.acquire(&intent).await?;
        let job = self
            .start_job
            .execute(TranslationRunRequest {
                project_id: request.project_id.clone(),
                translation_id: request.translation_id,
            })
            .await?;
        let result_id = self
            .run_job
            .execute(
                ExecuteTranslationRequest {
                    project_id: request.project_id.clone(),
                    translation_id: request.translation_id,
                    server_url: lease.server_url().into(),
                },
                job.id(),
            )
            .await?;
        drop(lease);
        self.publish
            .execute(PublishTranslationRequest {
                project_id: request.project_id,
                translation_id: request.translation_id,
                result_id,
            })
            .await
    }
}
