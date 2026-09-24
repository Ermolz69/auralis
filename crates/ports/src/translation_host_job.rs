use async_trait::async_trait;
use domain::job::Job;
use uuid::Uuid;

use crate::active_translation_host_job::ActiveTranslationHostJob;
use crate::error::PortError;

#[async_trait]
pub trait TranslationHostJobStore: Send + Sync {
    async fn create_pending(
        &self,
        job: Job,
        translation_id: Uuid,
        run_id: Uuid,
    ) -> Result<Job, PortError>;

    async fn active_for_run(&self, run_id: Uuid) -> Result<Option<Job>, PortError>;

    async fn list_active(&self) -> Result<Vec<ActiveTranslationHostJob>, PortError>;

    async fn start(&self, job: &Job, expected_revision: u64) -> Result<(), PortError>;

    async fn finish(&self, job: &Job, expected_revision: u64) -> Result<(), PortError>;
}
