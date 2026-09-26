use super::parse_translation_id;
use crate::bootstrap::translation::TranslationUseCases;
use crate::dto::{
    error::{CommandError, parse_project_id},
    translation::TranslationPublicationViewDto,
};
use application::usecases::translation::GetTranslationPublicationRequest;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn get_translation_publication_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    translation_id: String,
    result_id: String,
    usecases: State<'_, Arc<TranslationUseCases>>,
) -> Result<TranslationPublicationViewDto, CommandError> {
    crate::observability::command::observe("get_translation_publication_cmd", async {
        usecases
            .publication
            .execute(GetTranslationPublicationRequest {
                project_id: parse_project_id(&project_id)?,
                translation_id: parse_translation_id(&translation_id)?,
                result_id: parse_translation_id(&result_id)?,
            })
            .await
            .map(Into::into)
            .map_err(CommandError::from)
    })
    .await
}
