use super::parse_translation_id;
use crate::bootstrap::translation::TranslationUseCases;
use crate::dto::{
    error::{CommandError, parse_project_id},
    translation::EditHistoricalTranslationResultDto,
};
use application::usecases::translation::EditHistoricalTranslationRequest;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn edit_historical_translation_result_cmd(
    request: tauri::ipc::Request<'_>,
    input: EditHistoricalTranslationResultDto,
    usecases: State<'_, Arc<TranslationUseCases>>,
) -> Result<String, CommandError> {
    crate::observability::command::observe("edit_historical_translation_result_cmd", async {
        #[cfg(feature = "native-e2e")]
        crate::commands::native_e2e_historical_request::capture(&input).await?;
        let result_id = usecases
            .historical_edit
            .execute(EditHistoricalTranslationRequest {
                project_id: parse_project_id(&input.project_id)?,
                translation_id: parse_translation_id(&input.translation_id)?,
                base_result_id: parse_translation_id(&input.base_result_id)?,
                result_id: parse_translation_id(&input.result_id)?,
                expected_head_result_id: parse_translation_id(&input.expected_head_result_id)?,
                expected_link_revision: input.expected_link_revision,
                segment_id: input.segment_id,
                lines: input.lines,
            })
            .await
            .map_err(CommandError::from)?;
        Ok(result_id.to_string())
    })
    .await
}
