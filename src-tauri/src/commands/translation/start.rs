use super::parse_translation_id;
use crate::bootstrap::translation::TranslationUseCases;
use crate::dto::error::{CommandError, parse_project_id};
use crate::dto::translation::TranslationPublicationDto;
use application::usecases::translation::TranslationRunRequest;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn start_translation_run_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    translation_id: String,
    usecases: State<'_, Arc<TranslationUseCases>>,
) -> Result<TranslationPublicationDto, CommandError> {
    crate::observability::command::observe("start_translation_run_cmd", async {
        let managed = usecases.managed.as_ref().ok_or_else(|| {
            CommandError::NotFound("Translation model runtime is not installed".into())
        })?;
        let publication = managed
            .execute(TranslationRunRequest {
                project_id: parse_project_id(&project_id)?,
                translation_id: parse_translation_id(&translation_id)?,
            })
            .await
            .map_err(CommandError::from)?;
        Ok(publication.into())
    })
    .await
}
