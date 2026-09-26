use super::{
    ManagedTranslationAttempt, PublishTranslationResultUseCase, RunTranslationHostJobUseCase,
    StartTranslationJobUseCase, TranslationRunRequest,
};
use crate::error::ApplicationError;
use domain::job::Job;
use ports::error::PortError;
use ports::translation_links::{TranslationLinkStore, TranslationPublicationRecord};
use ports::translation_model_runtime::TranslationModelRuntimePort;
use ports::translation_run_control::{TranslationRunControlPort, TranslationRunPhase};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
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
        self.admit(request).await?.execute().await
    }

    pub async fn active_running_job(
        &self,
        request: &TranslationRunRequest,
    ) -> Result<Option<(Job, Uuid)>, ApplicationError> {
        self.start_job.active_running(request).await
    }

    pub async fn admit(
        &self,
        request: TranslationRunRequest,
    ) -> Result<ManagedTranslationAttempt, ApplicationError> {
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
        let start_guard = self.control.capture_start(&link).await?;
        if start_guard.run_id != run_id {
            return Err(PortError::Cancelled.into());
        }
        let acquisition = self.runtime.acquire(&intent).await;
        let current_link = self
            .links
            .get(request.translation_id)
            .await?
            .ok_or(PortError::Cancelled)?;
        if current_link.project_id != link.project_id
            || current_link.active_run_id != Some(run_id)
            || self.control.capture_start(&current_link).await? != start_guard
        {
            return Err(PortError::Cancelled.into());
        }
        let lease = acquisition?;
        let job = self
            .start_job
            .execute_for_run(
                TranslationRunRequest {
                    project_id: request.project_id.clone(),
                    translation_id: request.translation_id,
                },
                run_id,
            )
            .await?;
        Ok(ManagedTranslationAttempt::new(
            request,
            start_guard,
            job,
            lease,
            self.run_job.clone(),
            self.publish.clone(),
        ))
    }
}
