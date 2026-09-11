use crate::bootstrap::usecases::AppUseCases;
use crate::dto::artifact::{ArtifactDto, ArtifactKindDto};
use crate::dto::error::{CommandError, parse_artifact_id, parse_project_id};
use application::usecases::artifact::list_project_artifacts::ListProjectArtifactsRequest;
use application::usecases::artifact::resolve_path::ResolveArtifactPathRequest;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn list_project_artifacts_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    kind: Option<ArtifactKindDto>,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<Vec<ArtifactDto>, CommandError> {
    crate::observability::command::observe("list_project_artifacts_cmd", async {
        let parsed_project_id = parse_project_id(&project_id)?;

        let artifacts = usecases
            .list_project_artifacts
            .execute(ListProjectArtifactsRequest {
                project_id: parsed_project_id,
                kind: kind.map(Into::into),
            })
            .await
            .map_err(CommandError::from)?;

        Ok(artifacts.iter().map(ArtifactDto::from).collect())
    })
    .await
}

#[tauri::command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn resolve_artifact_path_cmd(
    request: tauri::ipc::Request<'_>,
    artifact_id: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<String, CommandError> {
    crate::observability::command::observe("resolve_artifact_path_cmd", async {
        let id = parse_artifact_id(&artifact_id)?;

        let req = ResolveArtifactPathRequest { artifact_id: id };

        let res = usecases
            .resolve_artifact_path
            .execute(req)
            .await
            .map_err(CommandError::from)?;

        Ok(res.absolute_path.to_string_lossy().into_owned())
    })
    .await
}
