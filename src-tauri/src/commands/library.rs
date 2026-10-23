use crate::dto::error::CommandError;
use application::usecases::library::LibraryUseCase;
use domain::library::LibraryState;
use std::sync::Arc;

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
