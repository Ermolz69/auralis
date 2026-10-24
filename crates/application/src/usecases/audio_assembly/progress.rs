use super::AudioAssemblyUseCases;
use crate::error::ApplicationError;
use domain::{
    dubbing::DubbingPipelineStage,
    job::{Job, JobProgress},
};
use ports::job_scheduler::JobLifecycleEventKind;
use uuid::Uuid;

impl AudioAssemblyUseCases {
    pub(super) async fn stage(
        &self,
        job: &mut Job,
        attempt: Uuid,
        percent: u8,
        stage: DubbingPipelineStage,
        message: &str,
    ) -> Result<(), ApplicationError> {
        let revision = job.revision();
        job.advance(
            stage,
            JobProgress {
                percent,
                message: message.into(),
                current_step: None,
                processed_items: None,
                total_items: None,
            },
        )?;
        self.store.update(job, attempt, revision).await?;
        self.emit(JobLifecycleEventKind::Progressed, job);
        Ok(())
    }
}
