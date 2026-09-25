use super::package_error;
use crate::dto::error::CommandError;
use adapters_translate::{TranslationPackageService, TranslationPackageStatus};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn get_translation_package_status_cmd(
    packages: State<'_, Arc<TranslationPackageService>>,
) -> Result<TranslationPackageStatus, CommandError> {
    let packages = packages.inner().clone();
    tokio::task::spawn_blocking(move || packages.status())
        .await
        .map_err(|_| CommandError::Internal("Model package status failed".into()))?
        .map_err(package_error::map)
}
