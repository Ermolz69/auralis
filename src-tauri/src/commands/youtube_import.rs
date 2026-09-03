use crate::{
    bootstrap::usecases::AppUseCases,
    dto::{
        error::{parse_project_id, CommandError},
        project::ProjectDto,
    },
};
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
pub async fn list_pending_youtube_imports_cmd(
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<Vec<PendingYoutubeImportDto>, CommandError> {
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
}

#[tauri::command]
pub async fn resume_youtube_import_cmd(
    project_id: String,
    app: tauri::AppHandle,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<ProjectDto, CommandError> {
    let id = parse_project_id(&project_id)?;
    let response = usecases
        .create_project_from_youtube
        .resume(&id)
        .await
        .map_err(CommandError::from)?;
    let _ = app.emit(
        "project-updated",
        serde_json::json!({"projectId": project_id}),
    );
    Ok(ProjectDto::from(&response.project))
}

#[tauri::command]
pub async fn discard_youtube_import_cmd(
    project_id: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<(), CommandError> {
    usecases
        .create_project_from_youtube
        .discard(&parse_project_id(&project_id)?)
        .await
        .map_err(CommandError::from)
}
