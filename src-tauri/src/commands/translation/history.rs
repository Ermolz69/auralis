use super::parse_translation_id;
use crate::bootstrap::translation::TranslationUseCases;
use crate::dto::error::{CommandError, parse_project_id};
use crate::dto::translation::TranslationHistoryPageDto;
use application::usecases::translation::ListTranslationHistoryRequest;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn list_translation_history_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    translation_id: String,
    before_result_id: Option<String>,
    limit: u32,
    usecases: State<'_, Arc<TranslationUseCases>>,
) -> Result<TranslationHistoryPageDto, CommandError> {
    crate::observability::command::observe("list_translation_history_cmd", async {
        usecases
            .history
            .execute(ListTranslationHistoryRequest {
                project_id: parse_project_id(&project_id)?,
                translation_id: parse_translation_id(&translation_id)?,
                before_result_id: before_result_id
                    .as_deref()
                    .map(parse_translation_id)
                    .transpose()?,
                limit,
            })
            .await
            .map(Into::into)
            .map_err(CommandError::from)
    })
    .await
}
