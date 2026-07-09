use adapters_storage::memory::InMemoryProjectRepository;

use adapters_ytdlp::ytdlp::YtDlpAdapter;

use application::usecases::project::create::{CreateProjectRequest, CreateProjectUseCase};
use application::usecases::project::create_from_youtube::{
    CreateProjectFromYoutubeRequest, CreateProjectFromYoutubeUseCase,
};
use application::usecases::project::get::{GetProjectRequest, GetProjectUseCase};

use ports::job_scheduler::JobSchedulerPort;
use ports::repository::ProjectRepository;
use std::sync::Arc;
use tauri::{command, AppHandle, State};

use crate::dto::error::CommandError;
use crate::dto::project::{CreateProjectResponse, ProjectDto, TranscriptDto};
pub(crate) fn get_ytdlp_adapter(app: &AppHandle) -> YtDlpAdapter {
    let candidates = crate::media_tools::resolve_ytdlp_candidates(app);
    YtDlpAdapter::new(candidates)
}

#[command]
pub async fn create_project_cmd(
    title: String,
    project_repo: State<'_, InMemoryProjectRepository>,
) -> Result<ProjectDto, CommandError> {
    let create_use_case = CreateProjectUseCase::new(project_repo.inner().clone());
    let req = CreateProjectRequest { title };
    let create_res = create_use_case
        .execute(req)
        .await
        .map_err(CommandError::from)?;

    Ok(ProjectDto::from(&create_res.project))
}

#[command]
pub async fn create_project_from_youtube_cmd(
    url: String,
    app: AppHandle,
    state: State<'_, Arc<dyn JobSchedulerPort>>,
    project_repo: State<'_, InMemoryProjectRepository>,
) -> Result<CreateProjectResponse, CommandError> {
    let ytdlp_adapter = get_ytdlp_adapter(&app);
    let use_case = CreateProjectFromYoutubeUseCase::new(
        project_repo.inner().clone(),
        ytdlp_adapter,
        state.inner().clone(),
    );

    let req = CreateProjectFromYoutubeRequest { url };
    let response = use_case.execute(req).await.map_err(CommandError::from)?;

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
) -> Result<Option<TranscriptDto>, CommandError> {
    let pid: domain::project::ProjectId = project_id
        .parse()
        .map_err(|e| CommandError::Validation(format!("Invalid project id: {}", e)))?;

    let project = project_repo
        .inner()
        .get(&pid)
        .await
        .map_err(|e| CommandError::Repository(e.to_string()))?;

    if let Some(project) = project {
        if let Some(transcript) = project.transcript() {
            return Ok(Some(TranscriptDto::from(transcript)));
        }
        Ok(None)
    } else {
        Err(CommandError::NotFound("Project not found".into()))
    }
}

#[command]
pub async fn get_project_cmd(
    project_id: String,
    _app: AppHandle,
    project_repo: State<'_, InMemoryProjectRepository>,
) -> Result<ProjectDto, CommandError> {
    let pid: domain::project::ProjectId = project_id
        .parse()
        .map_err(|e| CommandError::Validation(format!("Invalid project id: {}", e)))?;

    let use_case = GetProjectUseCase::new(project_repo.inner().clone());
    let req = GetProjectRequest { project_id: pid };

    let res = use_case.execute(req).await.map_err(CommandError::from)?;
    Ok(ProjectDto::from(&res.project))
}
