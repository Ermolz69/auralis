use crate::bootstrap::translation::TranslationUseCases;
use crate::dto::error::{CommandError, parse_project_id};
use crate::dto::translation::ProjectTranslationSnapshotDto;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn get_project_translation_snapshot_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    usecases: State<'_, Arc<TranslationUseCases>>,
) -> Result<ProjectTranslationSnapshotDto, CommandError> {
    crate::observability::command::observe("get_project_translation_snapshot_cmd", async {
        usecases
            .snapshot
            .execute(&parse_project_id(&project_id)?)
            .await
            .map(Into::into)
            .map_err(CommandError::from)
    })
    .await
}
