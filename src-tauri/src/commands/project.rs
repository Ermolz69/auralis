use adapters_storage::memory::InMemoryProjectRepository;

use adapters_ytdlp::ytdlp::YtDlpAdapter;
use application::usecases::pipeline::start_mock::{
    StartMockPipelineRequest, StartMockPipelineUseCase,
};
use application::usecases::project::create::{CreateProjectRequest, CreateProjectUseCase};
use application::usecases::project::import_source::{
    ImportVideoSourceRequest, ImportVideoSourceUseCase,
};
use domain::media::MediaSource;
use jobs::manager::JobManager;
use ports::repository::ProjectRepository;
use tauri::{command, AppHandle, State};

use crate::dto::project::{CreateProjectResponse, ProjectDto, TranscriptDto};

pub(crate) fn get_ytdlp_adapter(app: &AppHandle) -> YtDlpAdapter {
    let candidates = crate::media_tools::resolve_ytdlp_candidates(app);
    YtDlpAdapter::new(candidates)
}

#[command]
pub async fn create_project_cmd(
    title: String,
    project_repo: State<'_, InMemoryProjectRepository>,
) -> Result<ProjectDto, String> {
    let create_use_case = CreateProjectUseCase::new(project_repo.inner().clone());
    let req = CreateProjectRequest { title };
    let create_res = create_use_case
        .execute(req)
        .await
        .map_err(|e| format!("{:?}", e))?;

    Ok(ProjectDto::from(&create_res.project))
}

#[command]
pub async fn create_project_from_youtube_cmd(
    url: String,
    app: AppHandle,
    state: State<'_, JobManager>,
    project_repo: State<'_, InMemoryProjectRepository>,
) -> Result<CreateProjectResponse, String> {
    let create_use_case = CreateProjectUseCase::new(project_repo.inner().clone());
    let req = CreateProjectRequest { title: url.clone() };
    let create_res = create_use_case
        .execute(req)
        .await
        .map_err(|e| format!("{:?}", e))?;

    let ytdlp_adapter = get_ytdlp_adapter(&app);
    let import_use_case =
        ImportVideoSourceUseCase::new(project_repo.inner().clone(), ytdlp_adapter);
    let source = MediaSource::YoutubeUrl { url: url.clone() };
    let req2 = ImportVideoSourceRequest {
        project_id: create_res.project.id().clone(),
        source,
    };
    let import_res = import_use_case
        .execute(req2)
        .await
        .map_err(|e| format!("{:?}", e))?;

    let mut proj = import_res.project;
    proj.mark_ready_for_processing()
        .map_err(|e| format!("{:?}", e))?;
    project_repo
        .inner()
        .save(&proj)
        .await
        .map_err(|e| format!("{:?}", e))?;

    let pipeline_use_case =
        StartMockPipelineUseCase::new(project_repo.inner().clone(), state.inner().clone());
    let req3 = StartMockPipelineRequest {
        project_id: proj.id().clone(),
    };
    let response = pipeline_use_case
        .execute(req3)
        .await
        .map_err(|e| format!("{:?}", e))?;

    Ok(CreateProjectResponse {
        project: ProjectDto::from(&response.project),
        job: response.job,
    })
}

#[command]
pub async fn get_transcript_cmd(
    project_id: String,
    _app: AppHandle,
    project_repo: State<'_, InMemoryProjectRepository>,
) -> Result<Option<TranscriptDto>, String> {
    let pid: domain::project::ProjectId = project_id
        .parse()
        .map_err(|e| format!("Invalid project id: {}", e))?;

    let project = project_repo
        .inner()
        .get(&pid)
        .await
        .map_err(|e| format!("{:?}", e))?;

    if let Some(project) = project {
        if let Some(transcript) = project.transcript() {
            return Ok(Some(TranscriptDto::from(transcript)));
        }
    }

    Ok(None)
}

#[command]
pub fn run_dubbing_cmd(video_url: String) -> Result<String, String> {
    application::commands::run_dubbing(video_url)
}
