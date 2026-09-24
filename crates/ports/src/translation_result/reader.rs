use super::VerifiedTranslationResult;
use crate::error::PortError;
use crate::translation_links::TranslationLink;
use async_trait::async_trait;
use std::path::Path;
use uuid::Uuid;

#[async_trait]
pub trait TranslationResultPort: Send + Sync {
    async fn verified_result(
        &self,
        source_path: &Path,
        link: &TranslationLink,
        result_id: Uuid,
    ) -> Result<VerifiedTranslationResult, PortError>;
}
