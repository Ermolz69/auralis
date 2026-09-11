use crate::{
    bootstrap::usecases::AppUseCases,
    dto::{
        error::{CommandError, parse_project_id},
        project::ProjectDto,
    },
};
use adapters_tauri::event_publisher::EVENT_PROJECT_UPDATED;
use std::sync::Arc;
use tauri::{Emitter, State};

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingYoutubeImportDto {
    project_id: String,
    title: String,
    state: String,
}

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn list_pending_youtube_imports_cmd(
    request: tauri::ipc::Request<'_>,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<Vec<PendingYoutubeImportDto>, CommandError> {
    crate::observability::command::observe("list_pending_youtube_imports_cmd", async {
        Ok(usecases
            .create_project_from_youtube
            .list_pending()
            .await
            .map_err(CommandError::from)?
            .into_iter()
            .map(|session| PendingYoutubeImportDto {
                project_id: session.project.id.to_string(),
                title: session.project.title,
                state: format!("{:?}", session.state),
            })
            .collect())
    })
    .await
}

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn resume_youtube_import_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    app: tauri::AppHandle,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<ProjectDto, CommandError> {
    crate::observability::command::observe("resume_youtube_import_cmd", async {
        let id = parse_project_id(&project_id)?;
        let response = usecases
            .create_project_from_youtube
            .resume(&id)
            .await
            .map_err(CommandError::from)?;
        let _ = app.emit(
            EVENT_PROJECT_UPDATED,
            serde_json::json!({"projectId": project_id}),
        );
        Ok(ProjectDto::from(&response.project))
    })
    .await
}

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn discard_youtube_import_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<(), CommandError> {
    crate::observability::command::observe("discard_youtube_import_cmd", async {
        usecases
            .create_project_from_youtube
            .discard(&parse_project_id(&project_id)?)
            .await
            .map_err(CommandError::from)
    })
    .await
}
