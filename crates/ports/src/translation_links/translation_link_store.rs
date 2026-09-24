use super::{PublicationIntent, StartTranslationRun, TranslationIntent, TranslationLink};
use crate::error::PortError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[async_trait]
pub trait TranslationLinkStore: Send + Sync {
    async fn begin(&self, intent: TranslationIntent) -> Result<TranslationLink, PortError>;
    async fn get(&self, translation_id: Uuid) -> Result<Option<TranslationLink>, PortError>;
    async fn start_run(&self, request: StartTranslationRun) -> Result<TranslationLink, PortError>;
    async fn record_publication(&self, intent: PublicationIntent) -> Result<(), PortError>;
    async fn select_ready_result(
        &self,
        translation_id: Uuid,
        result_id: Uuid,
        updated_at: DateTime<Utc>,
    ) -> Result<TranslationLink, PortError>;
}
