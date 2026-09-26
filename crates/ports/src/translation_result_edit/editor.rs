use async_trait::async_trait;
use std::path::Path;
use uuid::Uuid;

use crate::error::PortError;
use crate::translation_links::{TranslationLink, TranslationRunIntent};

use super::TranslationResultEdit;

#[async_trait]
pub trait TranslationResultEditorPort: Send + Sync {
    async fn commit_branch_edit(
        &self,
        source_path: &Path,
        link: &TranslationLink,
        intent: &TranslationRunIntent,
        edit: crate::translation_branch_edit::TranslationBranchEdit,
    ) -> Result<Uuid, PortError>;

    async fn commit_edit(
        &self,
        source_path: &Path,
        link: &TranslationLink,
        intent: &TranslationRunIntent,
        edit: TranslationResultEdit,
    ) -> Result<Uuid, PortError>;
}
