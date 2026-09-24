use crate::error::PortError;
use crate::translation_links::{TranslationLink, TranslationRunIntent};
use async_trait::async_trait;
use std::path::Path;
use uuid::Uuid;

#[async_trait]
pub trait TranslationExecutionPort: Send + Sync {
    async fn execute(
        &self,
        source_path: &Path,
        link: &TranslationLink,
        intent: &TranslationRunIntent,
        server_url: &str,
    ) -> Result<Uuid, PortError>;
}
