use super::{
    AudioAssemblyUseCases,
    error::{conflict, invalid},
};
use crate::error::ApplicationError;
use domain::job::{JobError, JobStatus};
use ports::PortError;
use uuid::Uuid;

impl AudioAssemblyUseCases {
    pub(super) async fn finish_error(
        &self,
        id: Uuid,
        attempt: Uuid,
        error: &PortError,
        cancelled: bool,
    ) -> Result<(), ApplicationError> {
        if !cancelled && self.store.publication(id).await?.is_some() {
            return Ok(());
        }
        if cancelled {
            self.store.discard_unready(id, attempt).await?;
        }
        let Some(active) = self.store.active(id).await? else {
            return Ok(());
        };
        if active.attempt_id != attempt {
            return Err(conflict("Stale assembly failure").into());
        }
        let mut job = active.job;
        if matches!(
            job.status(),
            JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
        ) {
            self.store.close_interrupted().await?;
            return Ok(());
        }
        let revision = job.revision();
        if cancelled {
            job.cancel()?;
        } else {
            job.mark_failed(JobError::new(
                "audio_assembly_failed",
                error.to_string(),
                false,
            ))?;
        }
        self.store.update(&job, attempt, revision).await?;
        self.emit(
            if cancelled {
                ports::job_scheduler::JobLifecycleEventKind::Cancelled
            } else {
                ports::job_scheduler::JobLifecycleEventKind::Failed
            },
            &job,
        );
        Ok(())
    }

    pub async fn recover(&self) -> Result<(), ApplicationError> {
        for active in self.store.recovery_attempts().await? {
            let publication = self.store.publication(active.assembly_id).await?;
            if let Some(publication) = publication {
                if publication.ready {
                    continue;
                }
                if active.job.status() == &JobStatus::Running {
                    if let Some(job) = self.store.complete_publication(active.assembly_id).await? {
                        self.emit(ports::job_scheduler::JobLifecycleEventKind::Completed, &job);
                    }
                    continue;
                }
                self.store
                    .discard_unready(active.assembly_id, active.attempt_id)
                    .await?;
            }
            let cancelled = matches!(
                active.job.status(),
                JobStatus::Cancelled | JobStatus::Cancelling
            );
            self.finish_error(
                active.assembly_id,
                active.attempt_id,
                &invalid("Auralis stopped during audio assembly; resume explicitly"),
                cancelled,
            )
            .await?;
        }
        self.store.close_interrupted().await?;
        Ok(())
    }
}
