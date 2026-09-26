use super::{TranslationRunRequest, TranslationRunStatus, TranslationRunUseCase};
use crate::error::ApplicationError;
use domain::job::JobKind;
use ports::{error::PortError, translation_host_job::TranslationHostJobStore};
use std::sync::Arc;

pub struct GetTranslationRunStatusUseCase {
    run: TranslationRunUseCase,
    jobs: Arc<dyn TranslationHostJobStore>,
}

impl GetTranslationRunStatusUseCase {
    pub fn new(run: TranslationRunUseCase, jobs: Arc<dyn TranslationHostJobStore>) -> Self {
        Self { run, jobs }
    }

    pub async fn execute(
        &self,
        request: TranslationRunRequest,
    ) -> Result<TranslationRunStatus, ApplicationError> {
        let project_id = request.project_id.clone();
        let snapshot = self.run.status(request).await?;
        let active = self.jobs.active_for_run(snapshot.run_id).await?;
        if active.as_ref().is_some_and(|job| {
            job.project_id() != &project_id || job.kind() != &JobKind::Translation
        }) {
            return Err(PortError::Conflict {
                resource: "translation host status".into(),
                message: "active host job ownership differs from the requested translation".into(),
            }
            .into());
        }
        Ok(TranslationRunStatus {
            snapshot,
            active_host_job_id: active.map(|job| job.id().clone()),
        })
    }
}
