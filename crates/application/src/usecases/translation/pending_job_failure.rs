use crate::error::ApplicationError;
use domain::job::{JobError, JobId, JobStatus};
use ports::translation_host_job::TranslationHostJobStore;
use uuid::Uuid;

const SCHEDULING_FAILURE_CODE: &str = "TRANSLATION_SCHEDULING_FAILED";
const SCHEDULING_FAILURE_MESSAGE: &str = "Translation worker could not be attached";

pub(super) async fn close_pending(
    jobs: &dyn TranslationHostJobStore,
    run_id: Uuid,
    job_id: &JobId,
) -> Result<(), ApplicationError> {
    let Some(mut job) = jobs.active_for_run(run_id).await? else {
        return Ok(());
    };
    if job.id() != job_id || job.status() != &JobStatus::Pending {
        return Ok(());
    }
    let expected_revision = job.revision();
    job.mark_failed(JobError::new(
        SCHEDULING_FAILURE_CODE,
        SCHEDULING_FAILURE_MESSAGE,
        true,
    ))?;
    jobs.finish(&job, expected_revision).await?;
    Ok(())
}
