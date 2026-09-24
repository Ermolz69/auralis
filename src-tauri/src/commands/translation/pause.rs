use super::parse_translation_id;
use crate::bootstrap::translation::TranslationUseCases;
use crate::dto::error::{CommandError, parse_project_id};
use application::usecases::translation::TranslationRunRequest;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn pause_translation_run_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    translation_id: String,
    usecases: State<'_, Arc<TranslationUseCases>>,
) -> Result<(), CommandError> {
    crate::observability::command::observe("pause_translation_run_cmd", async {
        usecases
            .run
            .pause(TranslationRunRequest {
                project_id: parse_project_id(&project_id)?,
                translation_id: parse_translation_id(&translation_id)?,
            })
            .await
            .map_err(CommandError::from)
    })
    .await
}
