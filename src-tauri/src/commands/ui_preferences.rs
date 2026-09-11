use crate::{
    bootstrap::usecases::AppUseCases,
    dto::{
        error::{CommandError, parse_project_id},
        ui_preferences::{LegacyPinDto, ProjectPinDto, ProjectPinsDto, StoredThemeDto},
    },
};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn get_color_theme_cmd(
    request: tauri::ipc::Request<'_>,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<Option<StoredThemeDto>, CommandError> {
    Ok(usecases.ui_preferences.get_theme().await?.map(Into::into))
}
#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn set_color_theme_cmd(
    request: tauri::ipc::Request<'_>,
    value: String,
    expected_revision: u64,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<StoredThemeDto, CommandError> {
    Ok(usecases
        .ui_preferences
        .set_theme(&value, expected_revision)
        .await?
        .into())
}
#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn import_color_theme_cmd(
    request: tauri::ipc::Request<'_>,
    value: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<StoredThemeDto, CommandError> {
    Ok(usecases.ui_preferences.import_theme(&value).await?.into())
}
#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn get_project_pins_cmd(
    request: tauri::ipc::Request<'_>,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<ProjectPinsDto, CommandError> {
    Ok(usecases.ui_preferences.get_pins().await?.into())
}
#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn set_project_pin_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    pinned: bool,
    expected_revision: u64,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<ProjectPinDto, CommandError> {
    Ok(usecases
        .ui_preferences
        .set_pin(&parse_project_id(&project_id)?, pinned, expected_revision)
        .await?
        .into())
}
#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn import_project_pins_cmd(
    request: tauri::ipc::Request<'_>,
    entries: Vec<LegacyPinDto>,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<ProjectPinsDto, CommandError> {
    let entries = entries
        .into_iter()
        .map(|pin| Ok((parse_project_id(&pin.project_id)?, pin.pinned)))
        .collect::<Result<Vec<_>, CommandError>>()?;
    Ok(usecases.ui_preferences.import_pins(&entries).await?.into())
}
