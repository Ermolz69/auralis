use super::TranslationModelLease;
use crate::cancellation::CancellationToken;
use crate::error::PortError;
use crate::translation_links::TranslationRunIntent;
use async_trait::async_trait;

#[async_trait]
pub trait TranslationModelRuntimePort: Send + Sync {
    async fn acquire(
        &self,
        intent: &TranslationRunIntent,
    ) -> Result<Box<dyn TranslationModelLease>, PortError> {
        self.acquire_with_control(intent, CancellationToken::new())
            .await
    }

    async fn acquire_with_control(
        &self,
        intent: &TranslationRunIntent,
        cancellation: CancellationToken,
    ) -> Result<Box<dyn TranslationModelLease>, PortError>;
}
