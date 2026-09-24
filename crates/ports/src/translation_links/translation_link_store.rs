use super::{
    PublicationIntent, ReadyPublication, StartTranslationRun, TranslationIntent, TranslationLink,
};
use crate::error::PortError;
use crate::storage::StagedArtifact;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::project::ProjectId;
use uuid::Uuid;

#[async_trait]
pub trait TranslationLinkStore: Send + Sync {
    async fn begin(&self, intent: TranslationIntent) -> Result<TranslationLink, PortError>;
    async fn get(&self, translation_id: Uuid) -> Result<Option<TranslationLink>, PortError>;
    async fn list_by_project(
        &self,
        project_id: &ProjectId,
    ) -> Result<Vec<TranslationLink>, PortError>;
    async fn list_active(&self) -> Result<Vec<TranslationLink>, PortError>;
    async fn list_ready_publications(
        &self,
        limit: usize,
    ) -> Result<Vec<ReadyPublication>, PortError>;
    async fn start_run(&self, request: StartTranslationRun) -> Result<TranslationLink, PortError>;
    async fn commit_staged_publication(
        &self,
        intent: PublicationIntent,
        staged: StagedArtifact,
    ) -> Result<(), PortError>;
    async fn select_ready_result(
        &self,
        translation_id: Uuid,
        result_id: Uuid,
        updated_at: DateTime<Utc>,
    ) -> Result<TranslationLink, PortError>;
}
