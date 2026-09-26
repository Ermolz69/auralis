use super::{
    begin, finalization, history, list, publication_row, ready_publications, row, run_intent,
    select_history, staged_publication, start_run,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::project::ProjectId;
use ports::error::PortError;
use ports::storage::StagedArtifact;
use ports::translation_links::{
    PublicationIntent, ReadyPublication, SelectTranslationHistoryResult, StartTranslationRun,
    TranslationHistoryPage, TranslationIntent, TranslationLink, TranslationLinkStore,
    TranslationPublicationRecord, TranslationRunIntent,
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
    async fn list_ready_history(
        &self,
        project_id: &ProjectId,
        translation_id: Uuid,
        before_result_id: Option<Uuid>,
        limit: u32,
    ) -> Result<TranslationHistoryPage, PortError> {
        history::list(
            &self.pool,
            project_id,
            translation_id,
            before_result_id,
            limit,
        )
        .await
    }

    async fn select_historical_result(
        &self,
        request: SelectTranslationHistoryResult,
    ) -> Result<TranslationLink, PortError> {
        select_history::run(&self.pool, request).await
    }
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

    async fn get_publication(
        &self,
        translation_id: Uuid,
        result_id: Uuid,
    ) -> Result<Option<TranslationPublicationRecord>, PortError> {
        publication_row::load(&self.pool, translation_id, result_id).await
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

    async fn list_recoverable(&self) -> Result<Vec<TranslationLink>, PortError> {
        list::recoverable(&self.pool).await
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

    async fn finalize_ready_publication(
        &self,
        translation_id: Uuid,
        result_id: Uuid,
        updated_at: DateTime<Utc>,
    ) -> Result<TranslationLink, PortError> {
        finalization::run(&self.pool, translation_id, result_id, updated_at).await
    }
}
