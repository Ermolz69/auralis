use crate::bootstrap::usecases::AppUseCases;
use crate::dto::error::{parse_artifact_id, parse_artifact_kind, parse_project_id, CommandError};
use application::usecases::artifact::list_project_artifacts::ListProjectArtifactsRequest;
use application::usecases::artifact::resolve_path::ResolveArtifactPathRequest;
use domain::media::Artifact;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn list_project_artifacts_cmd(
    project_id: String,
    kind: Option<String>,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<Vec<Artifact>, CommandError> {
    let parsed_project_id = parse_project_id(&project_id)?;

    let parsed_kind = match kind {
        Some(k) => Some(parse_artifact_kind(&k)?),
        None => None,
    };

    usecases
        .list_project_artifacts
        .execute(ListProjectArtifactsRequest {
            project_id: parsed_project_id,
            kind: parsed_kind,
        })
        .await
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn resolve_artifact_path_cmd(
    artifact_id: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<String, CommandError> {
    let id = parse_artifact_id(&artifact_id)?;

    let req = ResolveArtifactPathRequest { artifact_id: id };

    let res = usecases
        .resolve_artifact_path
        .execute(req)
        .await
        .map_err(CommandError::from)?;

    Ok(res.absolute_path.to_string_lossy().into_owned())
}
