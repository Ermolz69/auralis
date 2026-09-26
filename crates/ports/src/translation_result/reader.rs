use super::{TranslationComparisonPage, VerifiedTranslationResult};
use crate::error::PortError;
use crate::translation_links::{TranslationLink, TranslationRunIntent};
use async_trait::async_trait;
use std::path::Path;
use uuid::Uuid;

#[async_trait]
pub trait TranslationResultPort: Send + Sync {
    async fn verified_frozen_result(
        &self,
        source_path: &Path,
        link: &TranslationLink,
        intent: &TranslationRunIntent,
        result_id: Uuid,
    ) -> Result<VerifiedTranslationResult, PortError>;

    async fn verified_result(
        &self,
        source_path: &Path,
        link: &TranslationLink,
        expected_run_id: Uuid,
        result_id: Uuid,
    ) -> Result<VerifiedTranslationResult, PortError>;

    async fn comparison_page(
        &self,
        source_path: &Path,
        link: &TranslationLink,
        expected_run_id: Uuid,
        result_id: Uuid,
        offset: u32,
        limit: u32,
    ) -> Result<TranslationComparisonPage, PortError>;
}
