use crate::state::{RuntimeArtifactIndex, RuntimeArtifactStore};
use application::usecases::artifact::list_project_artifacts::{
    ListProjectArtifactsRequest, ListProjectArtifactsUseCase,
};
use domain::media::{Artifact, ArtifactId, ArtifactKind};
use domain::project::ProjectId;
use std::str::FromStr;
use tauri::State;

#[tauri::command]
pub async fn list_project_artifacts_cmd(
    project_id: String,
    kind: Option<String>,
    artifact_index: State<'_, RuntimeArtifactIndex>,
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

    let use_case = ListProjectArtifactsUseCase::new(artifact_index.inner().clone());

    use_case
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
    artifact_index: State<'_, RuntimeArtifactIndex>,
    artifact_store: State<'_, RuntimeArtifactStore>,
) -> Result<String, String> {
    let id = ArtifactId::from_str(&artifact_id).map_err(|e| e.to_string())?;

    let artifact = artifact_index
        .get(&id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Artifact not found".to_string())?;

    let path = artifact_store
        .resolve_artifact(&artifact)
        .await
        .map_err(|e| e.to_string())?;

    // Only returning a safe absolute path that the UI can use e.g., with tauri convertFileSrc
    Ok(path.to_string_lossy().into_owned())
}
