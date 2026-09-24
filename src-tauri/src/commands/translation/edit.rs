use super::parse_translation_id;
use crate::bootstrap::translation::TranslationUseCases;
use crate::dto::error::{CommandError, parse_project_id};
use crate::dto::translation::EditTranslationResultDto;
use application::usecases::translation::EditTranslationResultRequest;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn edit_translation_result_cmd(
    request: tauri::ipc::Request<'_>,
    input: EditTranslationResultDto,
    usecases: State<'_, Arc<TranslationUseCases>>,
) -> Result<String, CommandError> {
    crate::observability::command::observe("edit_translation_result_cmd", async {
        let result_id = usecases
            .edit
            .execute(EditTranslationResultRequest {
                project_id: parse_project_id(&input.project_id)?,
                translation_id: parse_translation_id(&input.translation_id)?,
                base_result_id: parse_translation_id(&input.base_result_id)?,
                result_id: parse_translation_id(&input.result_id)?,
                segment_id: input.segment_id,
                lines: input.lines,
            })
            .await
            .map_err(CommandError::from)?;
        Ok(result_id.to_string())
    })
    .await
}
