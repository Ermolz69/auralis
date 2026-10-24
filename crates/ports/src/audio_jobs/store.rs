use super::{ActiveAssemblyAttempt, AudioAssemblyIntent, AudioAssemblyPublication};
use crate::{PortError, storage::StagedArtifact};
use async_trait::async_trait;
use domain::{job::Job, project::ProjectId};
use uuid::Uuid;

#[async_trait]
pub trait AudioAssemblyStore: Send + Sync {
    async fn register(
        &self,
        intent: &AudioAssemblyIntent,
    ) -> Result<AudioAssemblyIntent, PortError>;
    async fn get(&self, id: Uuid) -> Result<Option<AudioAssemblyIntent>, PortError>;
    async fn list_ids(&self, project: &ProjectId) -> Result<Vec<Uuid>, PortError>;
    async fn latest_job(&self, id: Uuid) -> Result<Option<Job>, PortError>;
    async fn active(&self, id: Uuid) -> Result<Option<ActiveAssemblyAttempt>, PortError>;
    async fn recovery_attempts(&self) -> Result<Vec<ActiveAssemblyAttempt>, PortError>;
    async fn create_pending(&self, id: Uuid, attempt_id: Uuid, job: Job) -> Result<Job, PortError>;
    async fn update(
        &self,
        job: &Job,
        attempt_id: Uuid,
        expected_revision: u64,
    ) -> Result<(), PortError>;
    async fn publication(&self, id: Uuid) -> Result<Option<AudioAssemblyPublication>, PortError>;
    async fn publish(
        &self,
        job: &Job,
        publication: &AudioAssemblyPublication,
        staged: &[StagedArtifact],
    ) -> Result<(), PortError>;
    async fn complete_publication(&self, id: Uuid) -> Result<Option<Job>, PortError>;
    async fn discard_unready(&self, id: Uuid, attempt_id: Uuid) -> Result<bool, PortError>;
    async fn close_interrupted(&self) -> Result<(), PortError>;
}
