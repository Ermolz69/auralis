use super::parse_translation_id;
use crate::bootstrap::translation::TranslationUseCases;
use crate::dto::{
    error::{CommandError, parse_project_id},
    translation::HistoricalTranslationEditContextDto,
};
use application::usecases::translation::GetHistoricalTranslationEditContextRequest;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn get_historical_translation_edit_context_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    translation_id: String,
    base_result_id: String,
    usecases: State<'_, Arc<TranslationUseCases>>,
) -> Result<HistoricalTranslationEditContextDto, CommandError> {
    crate::observability::command::observe("get_historical_translation_edit_context_cmd", async {
        usecases
            .edit_context
            .execute(GetHistoricalTranslationEditContextRequest {
                project_id: parse_project_id(&project_id)?,
                translation_id: parse_translation_id(&translation_id)?,
                base_result_id: parse_translation_id(&base_result_id)?,
            })
            .await
            .map(Into::into)
            .map_err(CommandError::from)
    })
    .await
}
