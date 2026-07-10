use crate::bootstrap::usecases::AppUseCases;
use application::usecases::artifact::list_project_artifacts::ListProjectArtifactsRequest;
use application::usecases::artifact::resolve_path::ResolveArtifactPathRequest;
use domain::media::{Artifact, ArtifactId, ArtifactKind};
use domain::project::ProjectId;
use std::str::FromStr;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn list_project_artifacts_cmd(
    project_id: String,
    kind: Option<String>,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<Vec<Artifact>, String> {
    let parsed_project_id = ProjectId::from_str(&project_id).map_err(|e| e.to_string())?;

    let parsed_kind = match kind {
        Some(k) => {
            let parsed: ArtifactKind =
                serde_json::from_str(&format!("\"{}\"", k)).map_err(|e| e.to_string())?;
            Some(parsed)
        }
        None => None,
    };

    usecases
        .list_project_artifacts
        .execute(ListProjectArtifactsRequest {
            project_id: parsed_project_id,
            kind: parsed_kind,
        })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn resolve_artifact_path_cmd(
    artifact_id: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<String, String> {
    let id = ArtifactId::from_str(&artifact_id).map_err(|e| e.to_string())?;

    let req = ResolveArtifactPathRequest { artifact_id: id };

    let res = usecases
        .resolve_artifact_path
        .execute(req)
        .await
        .map_err(|e| e.to_string())?;

    Ok(res.absolute_path.to_string_lossy().into_owned())
}
