use crate::error::PortError;
use crate::translation_inspection::TranslationSourceFormat;
use crate::translation_links::{TranslationLink, TranslationRunIntent};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::path::Path;
use uuid::Uuid;

#[async_trait]
pub trait TranslationRunRegistrationPort: Send + Sync {
    fn prepare_intent(
        &self,
        format: TranslationSourceFormat,
        translation_id: Uuid,
        run_id: Uuid,
        profile_manifest_json: String,
        created_at: DateTime<Utc>,
    ) -> Result<TranslationRunIntent, PortError>;

    async fn ensure_run(
        &self,
        source_path: &Path,
        link: &TranslationLink,
        intent: &TranslationRunIntent,
    ) -> Result<(), PortError>;
}
