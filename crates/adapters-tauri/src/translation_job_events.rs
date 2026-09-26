use std::sync::Arc;

use async_trait::async_trait;
use domain::job::{Job, JobStatus};
use ports::active_translation_host_job::ActiveTranslationHostJob;
use ports::error::PortError;
use ports::job_scheduler::{JobLifecycleEvent, JobLifecycleEventKind};
use ports::translation_host_job::TranslationHostJobStore;
use uuid::Uuid;

pub struct EventfulTranslationHostJobs {
    inner: Arc<dyn TranslationHostJobStore>,
    emit: Arc<dyn Fn(JobLifecycleEvent) + Send + Sync>,
}

impl EventfulTranslationHostJobs {
    pub fn new(
        inner: Arc<dyn TranslationHostJobStore>,
        emit: Arc<dyn Fn(JobLifecycleEvent) + Send + Sync>,
    ) -> Self {
        Self { inner, emit }
    }

    fn publish(&self, kind: JobLifecycleEventKind, job: &Job) {
        (self.emit)(JobLifecycleEvent {
            kind,
            job: job.to_snapshot().into(),
        });
    }
}

#[async_trait]
impl TranslationHostJobStore for EventfulTranslationHostJobs {
    async fn create_pending(
        &self,
        job: Job,
        translation_id: Uuid,
        run_id: Uuid,
    ) -> Result<Job, PortError> {
        let committed = self
            .inner
            .create_pending(job, translation_id, run_id)
            .await?;
        self.publish(JobLifecycleEventKind::Created, &committed);
        Ok(committed)
    }

    async fn active_for_run(&self, run_id: Uuid) -> Result<Option<Job>, PortError> {
        self.inner.active_for_run(run_id).await
    }

    async fn list_active(&self) -> Result<Vec<ActiveTranslationHostJob>, PortError> {
        self.inner.list_active().await
    }

    async fn start(&self, job: &Job, expected_revision: u64) -> Result<(), PortError> {
        self.inner.start(job, expected_revision).await?;
        self.publish(JobLifecycleEventKind::Started, job);
        Ok(())
    }

    async fn record_progress(&self, job: &Job, expected_revision: u64) -> Result<(), PortError> {
        self.inner.record_progress(job, expected_revision).await?;
        self.publish(JobLifecycleEventKind::Progressed, job);
        Ok(())
    }

    async fn finish(&self, job: &Job, expected_revision: u64) -> Result<(), PortError> {
        let kind = match job.status() {
            JobStatus::Completed => JobLifecycleEventKind::Completed,
            JobStatus::Cancelled => JobLifecycleEventKind::Cancelled,
            JobStatus::Failed => JobLifecycleEventKind::Failed,
            _ => {
                return Err(PortError::InvalidSource {
                    message: "translation host job must be terminal before finishing".into(),
                });
            }
        };
        self.inner.finish(job, expected_revision).await?;
        self.publish(kind, job);
        Ok(())
    }
}
