use super::{TranslationInspection, TranslationSourceFormat};
use crate::error::PortError;
use async_trait::async_trait;
use std::path::Path;

#[async_trait]
pub trait TranslationInspectorPort: Send + Sync {
    async fn inspect(
        &self,
        source_path: &Path,
        format: TranslationSourceFormat,
    ) -> Result<TranslationInspection, PortError>;
}
