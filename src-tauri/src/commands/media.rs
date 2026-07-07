use adapters_ffmpeg::ffprobe::FfprobeAdapter;
use adapters_storage::memory::InMemoryProjectRepository;
use application::usecases::media::probe_local::{ProbeLocalMediaRequest, ProbeLocalMediaUseCase};
use domain::project::ProjectId;
use ports::job_scheduler::JobSchedulerPort;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tauri::{command, AppHandle, State};

use crate::dto::media::MediaMetadataDto;
use crate::dto::project::ProjectDto;

fn get_ffprobe_adapter(app: &AppHandle) -> FfprobeAdapter {
    let candidates = crate::media_tools::resolve_ffprobe_candidates(app);
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

    let res = use_case.execute(req).await.map_err(|e| e.to_string())?;
    Ok((&res.metadata).into())
}

#[command]
pub async fn import_local_media_cmd(
    project_id: String,
    path: String,
    app: AppHandle,
    project_repo: State<'_, InMemoryProjectRepository>,
    job_scheduler: State<'_, Arc<dyn JobSchedulerPort>>,
) -> Result<ProjectDto, String> {
    use application::usecases::media::import_local_media::{
        ImportLocalMediaRequest, ImportLocalMediaUseCase,
    };

    let probe = get_ffprobe_adapter(&app);
    let use_case = ImportLocalMediaUseCase::new(
        project_repo.inner().clone(),
        probe,
        job_scheduler.inner().clone(),
    );

    let pid = ProjectId::from_str(&project_id).map_err(|e| e.to_string())?;

    let req = ImportLocalMediaRequest {
        project_id: pid,
        path: PathBuf::from(path),
    };

    let response = use_case.execute(req).await.map_err(|e| e.to_string())?;

    Ok((&response.project).into())
}
