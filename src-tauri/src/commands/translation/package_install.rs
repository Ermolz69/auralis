use super::{package_admission, package_error};
use crate::bootstrap::selected_translation_runtime::SelectedTranslationRuntime;
use crate::bootstrap::translation::TranslationUseCases;
use crate::dto::error::CommandError;
use adapters_translate::{TranslationPackageService, TranslationPackageStatus};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn install_translation_package_cmd(
    packages: State<'_, Arc<TranslationPackageService>>,
    selected_runtime: State<'_, SelectedTranslationRuntime>,
    usecases: State<'_, Arc<TranslationUseCases>>,
) -> Result<TranslationPackageStatus, CommandError> {
    let mut selected = selected_runtime.write().await;
    package_admission::ensure_no_active_jobs(&usecases).await?;
    let packages = packages.inner().clone();
    let (status, runtime) = tokio::task::spawn_blocking(move || {
        let status = packages.install()?;
        let runtime = if status.selected {
            packages.selected_runtime()?
        } else {
            None
        };
        Ok::<_, adapters_translate::TranslationPackageError>((status, runtime))
    })
    .await
    .map_err(|_| CommandError::Internal("Model installation failed".into()))?
    .map_err(package_error::map)?;
    if let Some(runtime) = runtime {
        *selected = Some(runtime);
    }
    Ok(status)
}
