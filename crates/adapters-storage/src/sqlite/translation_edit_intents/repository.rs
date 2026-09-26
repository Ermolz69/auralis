use super::{list, record, row};
use async_trait::async_trait;
use ports::error::PortError;
use ports::translation_edit_intents::{TranslationEditIntent, TranslationEditIntentStore};
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct SqliteTranslationEditIntentStore {
    pool: SqlitePool,
}
impl SqliteTranslationEditIntentStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TranslationEditIntentStore for SqliteTranslationEditIntentStore {
    async fn record(&self, intent: TranslationEditIntent) -> Result<(), PortError> {
        record::run(&self.pool, &intent).await
    }
    async fn get(&self, result_id: Uuid) -> Result<Option<TranslationEditIntent>, PortError> {
        row::load(&self.pool, result_id).await
    }
    async fn list_unpublished(
        &self,
        after_result_id: Option<Uuid>,
        limit: u32,
    ) -> Result<Vec<TranslationEditIntent>, PortError> {
        list::run(&self.pool, after_result_id, limit).await
    }
}
