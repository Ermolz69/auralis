use crate::bootstrap::usecases::AppUseCases;
use crate::dto::error::{CommandError, parse_project_id};
use crate::dto::media::MediaMetadataDto;
use crate::dto::project::ProjectDto;
use application::usecases::media::import_local_media::ImportLocalMediaRequest;
use application::usecases::media::probe_local::ProbeLocalMediaRequest;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{State, command};

#[command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn probe_local_media_cmd(
    request: tauri::ipc::Request<'_>,
    path: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<MediaMetadataDto, CommandError> {
    crate::observability::command::observe("probe_local_media_cmd", async {
        let req = ProbeLocalMediaRequest {
            path: PathBuf::from(path),
        };

        let res = usecases
            .probe_local_media
            .execute(req)
            .await
            .map_err(CommandError::from)?;
        Ok((&res.metadata).into())
    })
    .await
}

#[command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn import_local_media_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    path: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<ProjectDto, CommandError> {
    crate::observability::command::observe("import_local_media_cmd", async {
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
    })
    .await
}
