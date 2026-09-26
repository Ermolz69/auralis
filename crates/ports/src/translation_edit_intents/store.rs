use super::TranslationEditIntent;
use crate::error::PortError;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait TranslationEditIntentStore: Send + Sync {
    async fn record(&self, intent: TranslationEditIntent) -> Result<(), PortError>;
    async fn get(&self, result_id: Uuid) -> Result<Option<TranslationEditIntent>, PortError>;
    async fn list_unpublished(
        &self,
        after_result_id: Option<Uuid>,
        limit: u32,
    ) -> Result<Vec<TranslationEditIntent>, PortError>;
}
