use adapters_storage::memory::InMemoryProjectRepository;

use adapters_ytdlp::ytdlp::YtDlpAdapter;

use application::usecases::project::create::{CreateProjectRequest, CreateProjectUseCase};
use application::usecases::project::create_from_youtube::{
    CreateProjectFromYoutubeRequest, CreateProjectFromYoutubeUseCase,
};

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
    let ytdlp_adapter = get_ytdlp_adapter(&app);
    let use_case = CreateProjectFromYoutubeUseCase::new(
        project_repo.inner().clone(),
        ytdlp_adapter,
        state.inner().clone(),
    );

    let req = CreateProjectFromYoutubeRequest { url };
    let response = use_case
        .execute(req)
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
