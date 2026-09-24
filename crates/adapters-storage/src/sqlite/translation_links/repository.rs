use super::{
    begin, list, ready_publications, row, run_intent, select, staged_publication, start_run,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::project::ProjectId;
use ports::error::PortError;
use ports::storage::StagedArtifact;
use ports::translation_links::{
    PublicationIntent, ReadyPublication, StartTranslationRun, TranslationIntent, TranslationLink,
    TranslationLinkStore, TranslationRunIntent,
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
        begin::run(&self.pool, &intent, None).await
    }

    async fn begin_with_run_intent(
        &self,
        intent: TranslationIntent,
        run: TranslationRunIntent,
    ) -> Result<TranslationLink, PortError> {
        begin::run(&self.pool, &intent, Some(&run)).await
    }

    async fn run_intent(&self, run_id: Uuid) -> Result<Option<TranslationRunIntent>, PortError> {
        run_intent::load(&self.pool, run_id).await
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

    async fn list_ready_publications(
        &self,
        limit: usize,
    ) -> Result<Vec<ReadyPublication>, PortError> {
        ready_publications::list(&self.pool, limit).await
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
