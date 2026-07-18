use crate::error::ApplicationError;
use domain::job::{Job, JobError};
use domain::project::Project;
use ports::transaction::{CommitPipelineStartFailure, StorageUnitOfWork};
use std::sync::Arc;

pub(super) async fn compensate_start_failure(
    storage_uow: Arc<dyn StorageUnitOfWork>,
    project: &Project,
    job: &Job,
    error_msg: String,
) -> Result<(), ApplicationError> {
    let mut failed_project = project.clone();
    let mut failed_job = job.clone();
    let expected_job_revision = failed_job.revision();

    failed_project
        .apply_terminal_transition(failed_job.id(), domain::job::TerminalOutcome::Failed)?;
    failed_job.mark_failed(JobError::new(
        "SCHEDULING_FAILED",
        "Failed to schedule job.",
        false,
    ))?;

    let failure_cmd = CommitPipelineStartFailure {
        project: failed_project,
        job: failed_job,
        expected_job_revision,
    };

    storage_uow
        .commit_pipeline_start_failure(failure_cmd)
        .await
        .map_err(|e| ApplicationError::PipelineStartFailedNeedsRecovery {
            scheduling_error: error_msg,
            compensation_error: e.to_string(),
        })
}
