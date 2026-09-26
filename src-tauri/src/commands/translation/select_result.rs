use super::parse_translation_id;
use crate::bootstrap::translation::TranslationUseCases;
use crate::dto::error::{CommandError, parse_project_id};
use crate::dto::translation::TranslationLinkDto;
use application::usecases::translation::SelectTranslationResultRequest;
use ports::events::AppEventPublisher;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn select_translation_result_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    translation_id: String,
    result_id: String,
    expected_link_revision: u64,
    usecases: State<'_, Arc<TranslationUseCases>>,
    app: AppHandle,
) -> Result<TranslationLinkDto, CommandError> {
    crate::observability::command::observe("select_translation_result_cmd", async {
        let link = usecases
            .select_result
            .execute(SelectTranslationResultRequest {
                project_id: parse_project_id(&project_id)?,
                translation_id: parse_translation_id(&translation_id)?,
                result_id: parse_translation_id(&result_id)?,
                expected_link_revision,
            })
            .await
            .map_err(CommandError::from)?;
        if adapters_tauri::TauriEventPublisher::new(app)
            .publish_project_updated(&project_id)
            .await
            .is_err()
        {
            tracing::warn!(
                code = "HistoricalSelectionNotificationFailed",
                "selected result will be recovered by snapshot refresh"
            );
        }
        Ok(link.into())
    })
    .await
}
