use crate::error::ApplicationError;
use domain::project::ProjectId;
use ports::ui_preferences::{ProjectPin, ProjectPins, StoredTheme, UiPreferencesRepository};
use std::sync::Arc;

pub struct UiPreferencesUseCase {
    repository: Arc<dyn UiPreferencesRepository>,
}
impl UiPreferencesUseCase {
    pub fn new(repository: Arc<dyn UiPreferencesRepository>) -> Self {
        Self { repository }
    }
    pub async fn get_theme(&self) -> Result<Option<StoredTheme>, ApplicationError> {
        Ok(self.repository.get_theme().await?)
    }
    pub async fn set_theme(
        &self,
        value: &str,
        revision: u64,
    ) -> Result<StoredTheme, ApplicationError> {
        validate_theme(value)?;
        Ok(self.repository.set_theme(value, revision).await?)
    }
    pub async fn import_theme(&self, value: &str) -> Result<StoredTheme, ApplicationError> {
        validate_theme(value)?;
        Ok(self.repository.import_theme(value).await?)
    }
    pub async fn get_pins(&self) -> Result<ProjectPins, ApplicationError> {
        Ok(self.repository.get_pins().await?)
    }
    pub async fn set_pin(
        &self,
        project_id: &ProjectId,
        pinned: bool,
        revision: u64,
    ) -> Result<ProjectPin, ApplicationError> {
        Ok(self
            .repository
            .set_pin(project_id, pinned, revision)
            .await?)
    }
    pub async fn import_pins(
        &self,
        entries: &[(ProjectId, bool)],
    ) -> Result<ProjectPins, ApplicationError> {
        if entries.len() > 10_000 {
            return Err(ApplicationError::InvalidOperation {
                message: "Too many project preferences to migrate".into(),
            });
        }
        Ok(self.repository.import_pins(entries).await?)
    }
}
fn validate_theme(value: &str) -> Result<(), ApplicationError> {
    if ![
        "auralis",
        "abyss",
        "indigo",
        "ember",
        "violet",
        "frost",
        "polar",
        "sandstone",
    ]
    .contains(&value)
    {
        return Err(ApplicationError::InvalidOperation {
            message: "Unsupported color theme".into(),
        });
    }
    Ok(())
}
