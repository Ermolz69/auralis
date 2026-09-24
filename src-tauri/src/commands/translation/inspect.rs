use crate::bootstrap::translation::TranslationUseCases;
use crate::dto::error::{CommandError, parse_artifact_id, parse_project_id};
use crate::dto::translation::{TranslationFormatDto, TranslationInspectionDto};
use application::usecases::translation::InspectTranslationSourceRequest;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn inspect_translation_source_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    source_artifact_id: String,
    format: TranslationFormatDto,
    usecases: State<'_, Arc<TranslationUseCases>>,
) -> Result<TranslationInspectionDto, CommandError> {
    crate::observability::command::observe("inspect_translation_source_cmd", async {
        let inspection = usecases
            .inspect
            .execute(InspectTranslationSourceRequest {
                project_id: parse_project_id(&project_id)?,
                source_artifact_id: parse_artifact_id(&source_artifact_id)?,
                format: format.into(),
            })
            .await
            .map_err(CommandError::from)?;
        Ok(inspection.into())
    })
    .await
}
