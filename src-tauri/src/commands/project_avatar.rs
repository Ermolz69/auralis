use crate::{
    bootstrap::usecases::AppUseCases,
    dto::{
        error::{parse_project_id, CommandError},
        project_avatar::ProjectAvatarDto,
    },
};
use std::sync::Arc;
use tauri::{command, State};

#[command]
pub async fn get_project_avatar_cmd(
    project_id: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<ProjectAvatarDto, CommandError> {
    Ok(usecases
        .project_avatar
        .get(parse_project_id(&project_id)?)
        .await?
        .into())
}

#[command]
pub async fn set_project_avatar_cmd(
    project_id: String,
    data_url: Option<String>,
    only_if_missing: bool,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<ProjectAvatarDto, CommandError> {
    Ok(usecases
        .project_avatar
        .set(parse_project_id(&project_id)?, data_url, only_if_missing)
        .await?
        .into())
}
