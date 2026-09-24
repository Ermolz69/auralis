use super::{begin, publication, row, select, start_run};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ports::error::PortError;
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

    async fn start_run(&self, request: StartTranslationRun) -> Result<TranslationLink, PortError> {
        start_run::run(&self.pool, &request).await
    }

    async fn record_publication(&self, intent: PublicationIntent) -> Result<(), PortError> {
        publication::record(&self.pool, &intent).await
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
