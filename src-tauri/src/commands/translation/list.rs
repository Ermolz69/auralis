use crate::bootstrap::translation::TranslationUseCases;
use crate::dto::error::{CommandError, parse_project_id};
use crate::dto::translation::TranslationLinkDto;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn list_project_translations_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    usecases: State<'_, Arc<TranslationUseCases>>,
) -> Result<Vec<TranslationLinkDto>, CommandError> {
    crate::observability::command::observe("list_project_translations_cmd", async {
        let links = usecases
            .list
            .execute(&parse_project_id(&project_id)?)
            .await
            .map_err(CommandError::from)?;
        Ok(links.into_iter().map(TranslationLinkDto::from).collect())
    })
    .await
}
