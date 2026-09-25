use crate::error::PortError;
use domain::project::ProjectId;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait TranslationCleanupPort: Send + Sync {
    async fn delete_project_translation(
        &self,
        project_id: &ProjectId,
        translation_id: Uuid,
    ) -> Result<(), PortError>;
}
