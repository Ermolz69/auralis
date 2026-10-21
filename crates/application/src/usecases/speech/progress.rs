use super::{SpeechUseCases, invalid};
use async_trait::async_trait;
use domain::job::Job;
use ports::{
    error::PortError, job_scheduler::JobLifecycleEventKind, speech_run::SpeechCheckpoint,
    speech_workflow::SpeechProgressObserver,
};
use tokio::sync::Mutex;

pub(super) struct Progress {
    pub(super) cases: SpeechUseCases,
    pub(super) job: Mutex<Job>,
    pub(super) total: u32,
}

#[async_trait]
impl SpeechProgressObserver for Progress {
    async fn checkpoint(&self, checkpoint: SpeechCheckpoint) -> Result<(), PortError> {
        let mut job = self.job.lock().await;
        let previous = job.revision();
        let mut saved = job.clone();
        saved
            .report_speech_progress(u64::from(checkpoint.sequence), u64::from(self.total))
            .map_err(|e| invalid(e.to_string()))?;
        self.cases
            .store
            .record_checkpoint(&saved, &checkpoint, previous)
            .await?;
        *job = saved;
        self.cases.emit(JobLifecycleEventKind::Progressed, &job);
        Ok(())
    }
}
