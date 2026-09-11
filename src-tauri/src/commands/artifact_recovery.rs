use crate::{
    bootstrap::usecases::AppUseCases,
    dto::error::{CommandError, parse_project_id},
};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn list_artifact_recovery_cmd(
    request: tauri::ipc::Request<'_>,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<Vec<String>, CommandError> {
    Ok(usecases
        .artifact_recovery
        .list()
        .await?
        .into_iter()
        .map(|id| id.to_string())
        .collect())
}

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn retry_artifact_finalization_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<(), CommandError> {
    Ok(usecases
        .artifact_recovery
        .retry(&parse_project_id(&project_id)?)
        .await?)
}
