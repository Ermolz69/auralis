use crate::{
    bootstrap::usecases::AppUseCases,
    dto::{
        error::{CommandError, parse_project_id},
        project_avatar::ProjectAvatarDto,
    },
};
use std::sync::Arc;
use tauri::{State, command};

#[command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn get_project_avatar_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<ProjectAvatarDto, CommandError> {
    crate::observability::command::observe("get_project_avatar_cmd", async {
        Ok(usecases
            .project_avatar
            .get(parse_project_id(&project_id)?)
            .await?
            .into())
    })
    .await
}

#[command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn set_project_avatar_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    data_url: Option<String>,
    only_if_missing: bool,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<ProjectAvatarDto, CommandError> {
    crate::observability::command::observe("set_project_avatar_cmd", async {
        Ok(usecases
            .project_avatar
            .set(parse_project_id(&project_id)?, data_url, only_if_missing)
            .await?
            .into())
    })
    .await
}
