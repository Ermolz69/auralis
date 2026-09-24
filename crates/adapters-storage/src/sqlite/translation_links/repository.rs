use super::{begin, list, row, select, staged_publication, start_run};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::project::ProjectId;
use ports::error::PortError;
use ports::storage::StagedArtifact;
use ports::translation_links::{
    PublicationIntent, StartTranslationRun, TranslationIntent, TranslationLink,
    TranslationLinkStore,
};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteTranslationLinkStore {
    pool: SqlitePool,
}

impl SqliteTranslationLinkStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TranslationLinkStore for SqliteTranslationLinkStore {
    async fn begin(&self, intent: TranslationIntent) -> Result<TranslationLink, PortError> {
        begin::run(&self.pool, &intent).await
    }

    async fn get(&self, translation_id: Uuid) -> Result<Option<TranslationLink>, PortError> {
        row::load(&self.pool, translation_id).await
    }

    async fn list_by_project(
        &self,
        project_id: &ProjectId,
    ) -> Result<Vec<TranslationLink>, PortError> {
        list::by_project(&self.pool, project_id).await
    }

    async fn list_active(&self) -> Result<Vec<TranslationLink>, PortError> {
        list::active(&self.pool).await
    }

    async fn start_run(&self, request: StartTranslationRun) -> Result<TranslationLink, PortError> {
        start_run::run(&self.pool, &request).await
    }

    async fn commit_staged_publication(
        &self,
        intent: PublicationIntent,
        staged: StagedArtifact,
    ) -> Result<(), PortError> {
        staged_publication::commit(&self.pool, &intent, &staged).await
    }

    async fn select_ready_result(
        &self,
        translation_id: Uuid,
        result_id: Uuid,
        updated_at: DateTime<Utc>,
    ) -> Result<TranslationLink, PortError> {
        select::run(&self.pool, translation_id, result_id, updated_at).await
    }
}
