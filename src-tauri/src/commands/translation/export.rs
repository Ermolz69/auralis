use super::parse_translation_id;
use crate::bootstrap::translation::TranslationUseCases;
use crate::dto::error::{CommandError, parse_project_id};
use crate::dto::translation::TranslationExportReceiptDto;
use application::usecases::translation::ExportTranslationResultRequest;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn export_translation_result_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    translation_id: String,
    result_id: String,
    expected_result_sha256: String,
    destination: String,
    usecases: State<'_, Arc<TranslationUseCases>>,
) -> Result<TranslationExportReceiptDto, CommandError> {
    crate::observability::command::observe("export_translation_result_cmd", async {
        usecases
            .export
            .execute(ExportTranslationResultRequest {
                project_id: parse_project_id(&project_id)?,
                translation_id: parse_translation_id(&translation_id)?,
                result_id: parse_translation_id(&result_id)?,
                expected_result_sha256,
                destination: PathBuf::from(destination),
            })
            .await
            .map(Into::into)
            .map_err(CommandError::from)
    })
    .await
}
