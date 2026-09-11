use super::{ProjectPin, ProjectPins, StoredTheme};
use crate::error::PortError;
use async_trait::async_trait;
use domain::project::ProjectId;

#[async_trait]
pub trait UiPreferencesRepository: Send + Sync {
    async fn get_theme(&self) -> Result<Option<StoredTheme>, PortError>;
    async fn set_theme(
        &self,
        theme: &str,
        expected_revision: u64,
    ) -> Result<StoredTheme, PortError>;
    async fn import_theme(&self, theme: &str) -> Result<StoredTheme, PortError>;
    async fn get_pins(&self) -> Result<ProjectPins, PortError>;
    async fn set_pin(
        &self,
        project_id: &ProjectId,
        pinned: bool,
        expected_revision: u64,
    ) -> Result<ProjectPin, PortError>;
    async fn import_pins(&self, entries: &[(ProjectId, bool)]) -> Result<ProjectPins, PortError>;
}
