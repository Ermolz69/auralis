use super::package_error;
use crate::dto::error::CommandError;
use adapters_translate::{TranslationPackageService, TranslationPackageStatus};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn install_translation_package_cmd(
    packages: State<'_, Arc<TranslationPackageService>>,
) -> Result<TranslationPackageStatus, CommandError> {
    let packages = packages.inner().clone();
    tokio::task::spawn_blocking(move || packages.install())
        .await
        .map_err(|_| CommandError::Internal("Model installation failed".into()))?
        .map_err(package_error::map)
}
