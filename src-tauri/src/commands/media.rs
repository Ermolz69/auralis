use crate::bootstrap::usecases::AppUseCases;
use crate::dto::error::{parse_project_id, CommandError};
use crate::dto::media::MediaMetadataDto;
use crate::dto::project::ProjectDto;
use application::usecases::media::import_local_media::ImportLocalMediaRequest;
use application::usecases::media::probe_local::ProbeLocalMediaRequest;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{command, State};

#[command]
pub async fn probe_local_media_cmd(
    path: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<MediaMetadataDto, CommandError> {
    let req = ProbeLocalMediaRequest {
        path: PathBuf::from(path),
    };

    let res = usecases
        .probe_local_media
        .execute(req)
        .await
        .map_err(CommandError::from)?;
    Ok((&res.metadata).into())
}

#[command]
pub async fn import_local_media_cmd(
    project_id: String,
    path: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<ProjectDto, CommandError> {
    let pid = parse_project_id(&project_id)?;

    let req = ImportLocalMediaRequest {
        project_id: pid,
        path: PathBuf::from(path),
    };

    let response = usecases
        .import_local_media
        .execute(req)
        .await
        .map_err(CommandError::from)?;

    Ok((&response.project).into())
}
