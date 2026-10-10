use super::{
    ExecuteTranslationRequest, PublishTranslationRequest, PublishTranslationResultUseCase,
    RunTranslationHostJobUseCase, TranslationRunRequest,
};
use crate::error::ApplicationError;
use domain::job::Job;
use ports::translation_links::TranslationPublicationRecord;
use ports::translation_model_runtime::TranslationModelLease;
use ports::translation_start_guard::TranslationStartGuard;
use uuid::Uuid;

pub struct ManagedTranslationAttempt {
    request: TranslationRunRequest,
    start_guard: TranslationStartGuard,
    job: Job,
    lease: Box<dyn TranslationModelLease>,
    run_job: RunTranslationHostJobUseCase,
    publish: PublishTranslationResultUseCase,
}

impl ManagedTranslationAttempt {
    pub(super) fn new(
        request: TranslationRunRequest,
        start_guard: TranslationStartGuard,
        job: Job,
        lease: Box<dyn TranslationModelLease>,
        run_job: RunTranslationHostJobUseCase,
        publish: PublishTranslationResultUseCase,
    ) -> Self {
        Self {
            request,
            start_guard,
            job,
            lease,
            run_job,
            publish,
        }
    }

    pub fn job(&self) -> &Job {
        &self.job
    }

    pub fn run_id(&self) -> Uuid {
        self.start_guard.run_id
    }

    pub async fn execute(self) -> Result<TranslationPublicationRecord, ApplicationError> {
        #[cfg(feature = "native-e2e")]
        let run_id = self.start_guard.run_id;
        let run_result = self
            .run_job
            .execute(
                ExecuteTranslationRequest {
                    project_id: self.request.project_id.clone(),
                    translation_id: self.request.translation_id,
                    server_url: self.lease.server_url().into(),
                    start_guard: Some(self.start_guard),
                },
                self.job.id(),
            )
            .await;
        self.lease.stop().await?;
        let result_id = run_result?;
        #[cfg(feature = "native-e2e")]
        super::native_e2e_result_gap::hold_if_requested(run_id, result_id).await?;
        self.publish
            .execute(PublishTranslationRequest {
                project_id: self.request.project_id,
                translation_id: self.request.translation_id,
                result_id,
            })
            .await
    }
}
