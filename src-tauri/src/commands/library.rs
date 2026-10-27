use crate::dto::error::CommandError;
use application::usecases::library::LibraryUseCase;
use domain::library::LibraryState;
use std::sync::Arc;

#[tauri::command]
pub async fn get_tool_context_cmd(
    project_id: String,
    asset_id: String,
    service: tauri::State<'_, Arc<dyn ports::media_project::MediaProjectStore>>,
) -> Result<ports::media_project::ToolMaterialContext, CommandError> {
    crate::observability::command::observe("get_tool_context_cmd", async {
        let project = service.structure(&project_id).await?;
        let asset = project["assets"]
            .as_array()
            .and_then(|items| items.iter().find(|a| a["id"] == asset_id))
            .ok_or_else(|| {
                CommandError::Io(
                    "Материал ещё не перенесён в структуру этого проекта. Повторите добавление."
                        .into(),
                )
            })?;
        Ok(ports::media_project::ToolMaterialContext {
            project_id,
            asset_id: asset_id.clone(),
            version: asset["sha256"].as_str().map(str::to_owned),
            artifact_id: asset["artifact_id"].as_str().map(str::to_owned),
            track_id: project["tracks"]
                .as_array()
                .and_then(|items| items.iter().find(|t| t["asset_id"] == asset_id))
                .and_then(|t| t["id"].as_str())
                .map(str::to_owned),
        })
    })
    .await
}

#[tauri::command]
pub async fn attach_library_asset_cmd(
    operation_id: String,
    project_id: String,
    asset_id: String,
    duplicate: bool,
    service: tauri::State<'_, Arc<dyn ports::media_project::MediaProjectStore>>,
) -> Result<LibraryState, CommandError> {
    crate::observability::command::observe("attach_library_asset_cmd", async {
        Ok(service
            .attach(&operation_id, &project_id, &asset_id, duplicate)
            .await?)
    })
    .await
}
#[tauri::command]
pub async fn get_media_operation_cmd(
    operation_id: String,
    service: tauri::State<'_, Arc<dyn ports::media_project::MediaProjectStore>>,
) -> Result<Option<ports::media_project::MediaOperation>, CommandError> {
    crate::observability::command::observe("get_media_operation_cmd", async {
        Ok(service.operation(&operation_id).await?)
    })
    .await
}
#[tauri::command]
pub async fn cancel_media_operation_cmd(
    operation_id: String,
    service: tauri::State<'_, Arc<dyn ports::media_project::MediaProjectStore>>,
) -> Result<(), CommandError> {
    crate::observability::command::observe("cancel_media_operation_cmd", async {
        Ok(service.cancel(&operation_id).await?)
    })
    .await
}
#[tauri::command]
pub async fn get_media_project_cmd(
    project_id: String,
    service: tauri::State<'_, Arc<dyn ports::media_project::MediaProjectStore>>,
) -> Result<std::collections::BTreeMap<String, serde_json::Value>, CommandError> {
    crate::observability::command::observe("get_media_project_cmd", async {
        serde_json::from_value(service.structure(&project_id).await?)
            .map_err(|error| CommandError::Io(format!("Invalid media project object: {error}")))
    })
    .await
}
#[tauri::command]
pub async fn validate_media_project_cmd(
    project_id: String,
    service: tauri::State<'_, Arc<dyn ports::media_project::MediaProjectStore>>,
) -> Result<(), CommandError> {
    crate::observability::command::observe("validate_media_project_cmd", async {
        Ok(service.validate(&project_id).await?)
    })
    .await
}

#[tauri::command]
pub async fn read_library_text_cmd(
    path: String,
    service: tauri::State<'_, Arc<LibraryUseCase>>,
) -> Result<String, CommandError> {
    crate::observability::command::observe("read_library_text_cmd", async {
        Ok(service.read_text(&path).await?)
    })
    .await
}

#[tauri::command]
pub async fn get_library_cmd(
    service: tauri::State<'_, Arc<LibraryUseCase>>,
) -> Result<LibraryState, CommandError> {
    crate::observability::command::observe("get_library_cmd", async { Ok(service.load().await?) })
        .await
}

#[tauri::command]
pub async fn save_library_cmd(
    state: LibraryState,
    service: tauri::State<'_, Arc<LibraryUseCase>>,
) -> Result<LibraryState, CommandError> {
    crate::observability::command::observe("save_library_cmd", async {
        Ok(service.save(state).await?)
    })
    .await
}
