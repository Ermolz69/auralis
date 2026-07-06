use adapters_ffmpeg::ffprobe::FfprobeAdapter;
use adapters_storage::memory::InMemoryProjectRepository;
use application::usecases::media::probe_local::{ProbeLocalMediaRequest, ProbeLocalMediaUseCase};
use domain::project::ProjectId;
use std::path::PathBuf;
use std::str::FromStr;
use tauri::{command, AppHandle, State};

use crate::dto::media::MediaMetadataDto;
use crate::dto::project::ProjectDto;

fn get_ffprobe_adapter(_app: &AppHandle) -> FfprobeAdapter {
    let candidates = vec![
        PathBuf::from("src-tauri/binaries/ffprobe-x86_64-pc-windows-msvc.exe"),
        PathBuf::from("src-tauri/binaries/ffprobe-aarch64-apple-darwin"),
        PathBuf::from("ffprobe"),
        PathBuf::from("ffprobe.exe"),
    ];
    FfprobeAdapter::new(candidates)
}

#[command]
pub async fn probe_local_media_cmd(
    path: String,
    app: AppHandle,
    project_repo: State<'_, InMemoryProjectRepository>,
) -> Result<MediaMetadataDto, String> {
    let probe = get_ffprobe_adapter(&app);
    let use_case = ProbeLocalMediaUseCase::new(project_repo.inner().clone(), probe);

    let req = ProbeLocalMediaRequest {
        project_id: None,
        path: PathBuf::from(path),
    };

    let res = use_case
        .execute(req)
        .await
        .map_err(|e| format!("{:?}", e))?;
    Ok((&res.metadata).into())
}

#[command]
pub async fn import_local_media_cmd(
    project_id: String,
    path: String,
    app: AppHandle,
    project_repo: State<'_, InMemoryProjectRepository>,
) -> Result<ProjectDto, String> {
    let probe = get_ffprobe_adapter(&app);
    let use_case = ProbeLocalMediaUseCase::new(project_repo.inner().clone(), probe);

    let pid = ProjectId::from_str(&project_id).map_err(|e| e.to_string())?;

    let req = ProbeLocalMediaRequest {
        project_id: Some(pid),
        path: PathBuf::from(path),
    };

    let res = use_case
        .execute(req)
        .await
        .map_err(|e| format!("{:?}", e))?;

    if let Some(proj) = res.project {
        Ok((&proj).into())
    } else {
        Err("Failed to return project".into())
    }
}
