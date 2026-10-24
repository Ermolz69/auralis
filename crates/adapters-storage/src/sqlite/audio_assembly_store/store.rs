use super::{SqliteAudioAssemblyStore, queries};
use crate::sqlite::transaction_gateway as gateway;
use async_trait::async_trait;
use domain::{job::Job, project::ProjectId};
use ports::{
    PortError,
    audio_jobs::{
        ActiveAssemblyAttempt, AudioAssemblyIntent, AudioAssemblyPublication, AudioAssemblyStore,
    },
    storage::StagedArtifact,
};
use uuid::Uuid;

#[async_trait]
impl AudioAssemblyStore for SqliteAudioAssemblyStore {
    async fn register(
        &self,
        intent: &AudioAssemblyIntent,
    ) -> Result<AudioAssemblyIntent, PortError> {
        gateway::register_assembly(&self.pool, intent).await
    }
    async fn get(&self, id: Uuid) -> Result<Option<AudioAssemblyIntent>, PortError> {
        queries::get(&self.pool, id).await
    }
    async fn list_ids(&self, project: &ProjectId) -> Result<Vec<Uuid>, PortError> {
        queries::list_ids(&self.pool, project).await
    }
    async fn latest_job(&self, id: Uuid) -> Result<Option<Job>, PortError> {
        queries::latest_job(&self.pool, id).await
    }
    async fn active(&self, id: Uuid) -> Result<Option<ActiveAssemblyAttempt>, PortError> {
        Ok(queries::active(&self.pool, Some(id))
            .await?
            .into_iter()
            .next())
    }
    async fn recovery_attempts(&self) -> Result<Vec<ActiveAssemblyAttempt>, PortError> {
        queries::active(&self.pool, None).await
    }
    async fn create_pending(&self, id: Uuid, attempt_id: Uuid, job: Job) -> Result<Job, PortError> {
        gateway::create_assembly_attempt(&self.pool, id, attempt_id, job).await
    }
    async fn update(
        &self,
        job: &Job,
        attempt_id: Uuid,
        expected_revision: u64,
    ) -> Result<(), PortError> {
        gateway::update_assembly_attempt(&self.pool, job, attempt_id, expected_revision).await
    }
    async fn publication(&self, id: Uuid) -> Result<Option<AudioAssemblyPublication>, PortError> {
        queries::publication(&self.pool, id).await
    }
    async fn publish(
        &self,
        job: &Job,
        publication: &AudioAssemblyPublication,
        staged: &[StagedArtifact],
    ) -> Result<(), PortError> {
        gateway::publish_assembly(&self.pool, job, publication, staged).await
    }
    async fn complete_publication(&self, id: Uuid) -> Result<Option<Job>, PortError> {
        gateway::complete_assembly_publication(&self.pool, id).await
    }
    async fn discard_unready(&self, id: Uuid, attempt_id: Uuid) -> Result<bool, PortError> {
        gateway::discard_assembly_publication(&self.pool, id, attempt_id).await
    }
    async fn close_interrupted(&self) -> Result<(), PortError> {
        gateway::close_assembly_attempts(&self.pool).await
    }
}
