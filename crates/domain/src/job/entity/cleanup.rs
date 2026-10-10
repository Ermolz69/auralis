use super::Job;
use crate::{
    error::DomainError,
    job::{JobError, JobStatus},
};

impl Job {
    pub(super) fn clear_cleanup_attention(&mut self) {
        if self
            .error
            .as_ref()
            .is_some_and(|error| error.code == "CLEANUP_UNCONFIRMED")
        {
            self.error = None;
        }
        if self.progress.current_step.as_deref() == Some("cleanup_unconfirmed") {
            self.progress.current_step = None;
        }
    }

    /// Cleanup is not a terminal transition and does not authorize a new worker.
    pub fn record_unconfirmed_cleanup(&mut self) -> Result<(), DomainError> {
        if self.progress.current_step.as_deref() == Some("cleanup_unconfirmed") {
            return Ok(());
        }
        if !matches!(
            self.status,
            JobStatus::Pending | JobStatus::Running | JobStatus::Cancelling
        ) {
            return Ok(());
        }
        self.bump_revision()?;
        self.status = JobStatus::Cancelling;
        self.error = Some(JobError::new(
            "CLEANUP_UNCONFIRMED",
            "Executor cleanup needs attention; resources remain reserved",
            false,
        ));
        self.progress.message = "Cleanup needs attention".into();
        self.progress.current_step = Some("cleanup_unconfirmed".into());
        self.updated_at = chrono::Utc::now();
        Ok(())
    }
}
