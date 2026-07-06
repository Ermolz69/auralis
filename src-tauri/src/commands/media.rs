use adapters_ffmpeg::ffprobe::FfprobeAdapter;
use adapters_storage::memory::InMemoryProjectRepository;
use application::usecases::media::probe_local::{ProbeLocalMediaRequest, ProbeLocalMediaUseCase};
use application::usecases::pipeline::start_mock::{
    StartMockPipelineRequest, StartMockPipelineUseCase,
};
use domain::project::ProjectId;
use jobs::manager::JobManager;
use ports::repository::ProjectRepository;
use std::path::PathBuf;
use std::str::FromStr;
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
    job_manager: State<'_, JobManager>,
) -> Result<ProjectDto, String> {
    let probe = get_ffprobe_adapter(&app);
    let use_case = ProbeLocalMediaUseCase::new(project_repo.inner().clone(), probe);

    let pid = ProjectId::from_str(&project_id).map_err(|e| e.to_string())?;

    let req = ProbeLocalMediaRequest {
        project_id: Some(pid.clone()),
        path: PathBuf::from(path),
    };

    let res = use_case.execute(req).await.map_err(|e| e.to_string())?;

    if let Some(mut proj) = res.project {
        proj.mark_ready_for_processing()
            .map_err(|e| e.to_string())?;
        project_repo
            .inner()
            .save(&proj)
            .await
            .map_err(|e| e.to_string())?;

        let pipeline_use_case = StartMockPipelineUseCase::new(
            project_repo.inner().clone(),
            job_manager.inner().clone(),
        );
        let req2 = StartMockPipelineRequest {
            project_id: pid.clone(),
        };
        let pipeline_res = pipeline_use_case
            .execute(req2)
            .await
            .map_err(|e| e.to_string())?;

        Ok((&pipeline_res.project).into())
    } else {
        Err("Failed to return project".into())
    }
}
