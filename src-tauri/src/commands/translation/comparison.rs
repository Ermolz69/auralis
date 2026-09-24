use super::parse_translation_id;
use crate::bootstrap::translation::TranslationUseCases;
use crate::dto::error::{CommandError, parse_project_id};
use crate::dto::translation::TranslationComparisonPageDto;
use application::usecases::translation::GetTranslationComparisonRequest;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn get_translation_comparison_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    translation_id: String,
    result_id: String,
    offset: u32,
    limit: u32,
    usecases: State<'_, Arc<TranslationUseCases>>,
) -> Result<TranslationComparisonPageDto, CommandError> {
    crate::observability::command::observe("get_translation_comparison_cmd", async {
        let page = usecases
            .comparison
            .execute(GetTranslationComparisonRequest {
                project_id: parse_project_id(&project_id)?,
                translation_id: parse_translation_id(&translation_id)?,
                result_id: parse_translation_id(&result_id)?,
                offset,
                limit,
            })
            .await
            .map_err(CommandError::from)?;
        Ok(page.into())
    })
    .await
}
