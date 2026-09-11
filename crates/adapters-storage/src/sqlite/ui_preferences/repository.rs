use async_trait::async_trait;
use domain::project::ProjectId;
use ports::{
    error::PortError,
    ui_preferences::{ProjectPin, ProjectPins, StoredTheme, UiPreferencesRepository},
};
use sqlx::SqlitePool;

pub struct SqliteUiPreferences {
    pool: SqlitePool,
}
impl SqliteUiPreferences {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UiPreferencesRepository for SqliteUiPreferences {
    async fn get_theme(&self) -> Result<Option<StoredTheme>, PortError> {
        super::theme::get(&self.pool).await
    }
    async fn set_theme(
        &self,
        theme: &str,
        expected_revision: u64,
    ) -> Result<StoredTheme, PortError> {
        super::theme::set(&self.pool, theme, expected_revision).await
    }
    async fn import_theme(&self, theme: &str) -> Result<StoredTheme, PortError> {
        super::theme::import(&self.pool, theme).await
    }
    async fn get_pins(&self) -> Result<ProjectPins, PortError> {
        super::pins::get(&self.pool).await
    }
    async fn set_pin(
        &self,
        project_id: &ProjectId,
        pinned: bool,
        expected_revision: u64,
    ) -> Result<ProjectPin, PortError> {
        super::pins::set(&self.pool, project_id, pinned, expected_revision).await
    }
    async fn import_pins(&self, entries: &[(ProjectId, bool)]) -> Result<ProjectPins, PortError> {
        super::pins::import(&self.pool, entries).await
    }
}

pub(super) fn error(error: sqlx::Error) -> PortError {
    crate::sqlite::helpers::map_sqlite_error("ui_preferences", error)
}
pub(super) fn conflict() -> PortError {
    PortError::Conflict {
        resource: "UiPreferences".into(),
        message: "Preferences changed. Reload before retrying.".into(),
    }
}
