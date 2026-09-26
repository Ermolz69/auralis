use super::{package_admission, package_error};
use crate::bootstrap::selected_translation_runtime::SelectedTranslationRuntime;
use crate::bootstrap::translation::TranslationUseCases;
use crate::dto::error::CommandError;
use adapters_translate::{TranslationPackageService, TranslationPackageStatus};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn select_translation_package_cmd(
    release_id: String,
    backend: String,
    packages: State<'_, Arc<TranslationPackageService>>,
    selected_runtime: State<'_, SelectedTranslationRuntime>,
    usecases: State<'_, Arc<TranslationUseCases>>,
) -> Result<TranslationPackageStatus, CommandError> {
    let mut selected = selected_runtime.write().await;
    package_admission::ensure_no_active_jobs(&usecases).await?;
    let packages = packages.inner().clone();
    let (runtime, status) = tokio::task::spawn_blocking(move || {
        let runtime = packages.select(&release_id, &backend)?;
        Ok::<_, adapters_translate::TranslationPackageError>((runtime, packages.status()?))
    })
    .await
    .map_err(|_| CommandError::Internal("Model selection failed".into()))?
    .map_err(package_error::map)?;
    *selected = Some(runtime);
    Ok(status)
}
