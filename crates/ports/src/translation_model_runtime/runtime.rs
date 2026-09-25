use super::TranslationModelLease;
use crate::error::PortError;
use crate::translation_links::TranslationRunIntent;
use async_trait::async_trait;

#[async_trait]
pub trait TranslationModelRuntimePort: Send + Sync {
    async fn acquire(
        &self,
        intent: &TranslationRunIntent,
    ) -> Result<Box<dyn TranslationModelLease>, PortError>;
}
