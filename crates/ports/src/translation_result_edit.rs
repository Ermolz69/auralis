use async_trait::async_trait;
use std::path::Path;
use uuid::Uuid;

use crate::error::PortError;
use crate::translation_links::{TranslationLink, TranslationRunIntent};

pub struct TranslationResultEdit {
    pub base_result_id: Uuid,
    pub result_id: Uuid,
    pub segment_id: u32,
    pub lines: Vec<String>,
}

#[async_trait]
pub trait TranslationResultEditorPort: Send + Sync {
    async fn commit_edit(
        &self,
        source_path: &Path,
        link: &TranslationLink,
        intent: &TranslationRunIntent,
        edit: TranslationResultEdit,
    ) -> Result<Uuid, PortError>;
}
