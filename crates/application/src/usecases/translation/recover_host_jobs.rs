use super::TranslationHostRecoveryItem;
use crate::error::ApplicationError;
use domain::job::{JobError, JobKind, JobStatus};
use ports::active_translation_host_job::ActiveTranslationHostJob;
use ports::translation_host_job::TranslationHostJobStore;
use ports::translation_run_control::TranslationRunPhase;
use ports::translation_run_recovery::TranslationRunRecoveryPort;
use std::sync::Arc;

const INTERRUPTION_CODE: &str = "TRANSLATION_APP_RESTART";
const INTERRUPTION_MESSAGE: &str = "Translation worker stopped during application restart";

pub struct RecoverTranslationHostJobsUseCase {
    jobs: Arc<dyn TranslationHostJobStore>,
    runs: Arc<dyn TranslationRunRecoveryPort>,
}

impl RecoverTranslationHostJobsUseCase {
    pub fn new(
        jobs: Arc<dyn TranslationHostJobStore>,
        runs: Arc<dyn TranslationRunRecoveryPort>,
    ) -> Self {
        Self { jobs, runs }
    }

    pub async fn execute(&self) -> Result<Vec<TranslationHostRecoveryItem>, ApplicationError> {
        let mut outcomes = Vec::new();
        for active in self.jobs.list_active().await? {
            let job_id = active.job.id().clone();
            let run_id = active.run_id;
            outcomes.push(TranslationHostRecoveryItem {
                job_id,
                run_id,
                outcome: self.recover(active).await,
            });
        }
        Ok(outcomes)
    }

    async fn recover(&self, active: ActiveTranslationHostJob) -> Result<(), ApplicationError> {
        let mut job = active.job;
        if job.kind() != &JobKind::Translation
            || matches!(
                job.status(),
                JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
            )
        {
            return Err(ApplicationError::InvalidOperation {
                message: "Active translation association has an invalid job kind or state".into(),
            });
        }
        self.runs.recover_for_host(active.run_id, job.id()).await?;
        let phase = self.runs.phase(active.run_id).await?;
        let expected_revision = job.revision();
        match phase {
            TranslationRunPhase::Validated if job.status() == &JobStatus::Running => {
                job.mark_completed()?;
            }
            TranslationRunPhase::Paused => {
                job.cancel()?;
            }
            TranslationRunPhase::Requested if job.status() == &JobStatus::Pending => {
                job.cancel()?;
            }
            TranslationRunPhase::Running => {
                return Err(ApplicationError::InvalidOperation {
                    message: "Owned translation attempt remained running after recovery".into(),
                });
            }
            _ => {
                job.mark_failed(JobError::new(INTERRUPTION_CODE, INTERRUPTION_MESSAGE, true))?;
            }
        }
        self.jobs.finish(&job, expected_revision).await?;
        Ok(())
    }
}
