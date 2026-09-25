use super::{
    ExecuteTranslationRequest, PublishTranslationRequest, PublishTranslationResultUseCase,
    RunTranslationHostJobUseCase, TranslationRunRequest,
};
use crate::error::ApplicationError;
use domain::job::Job;
use ports::translation_links::TranslationPublicationRecord;
use ports::translation_model_runtime::TranslationModelLease;
use uuid::Uuid;

pub struct ManagedTranslationAttempt {
    request: TranslationRunRequest,
    run_id: Uuid,
    job: Job,
    lease: Box<dyn TranslationModelLease>,
    run_job: RunTranslationHostJobUseCase,
    publish: PublishTranslationResultUseCase,
}

impl ManagedTranslationAttempt {
    pub(super) fn new(
        request: TranslationRunRequest,
        run_id: Uuid,
        job: Job,
        lease: Box<dyn TranslationModelLease>,
        run_job: RunTranslationHostJobUseCase,
        publish: PublishTranslationResultUseCase,
    ) -> Self {
        Self {
            request,
            run_id,
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
        self.run_id
    }

    pub async fn execute(self) -> Result<TranslationPublicationRecord, ApplicationError> {
        let result_id = self
            .run_job
            .execute(
                ExecuteTranslationRequest {
                    project_id: self.request.project_id.clone(),
                    translation_id: self.request.translation_id,
                    server_url: self.lease.server_url().into(),
                },
                self.job.id(),
            )
            .await?;
        drop(self.lease);
        self.publish
            .execute(PublishTranslationRequest {
                project_id: self.request.project_id,
                translation_id: self.request.translation_id,
                result_id,
            })
            .await
    }
}
