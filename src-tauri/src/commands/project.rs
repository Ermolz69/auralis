use crate::bootstrap::usecases::AppUseCases;
use crate::state::RuntimeProjectRepository;

use application::usecases::pipeline::start_mock::StartMockPipelineRequest;
use application::usecases::project::create::CreateProjectRequest;
use application::usecases::project::create_from_youtube::CreateProjectFromYoutubeRequest;
use application::usecases::project::delete::DeleteProjectRequest;
use application::usecases::project::get::GetProjectRequest;

use std::sync::Arc;
use tauri::{command, State};

use crate::dto::error::CommandError;
use crate::dto::project::{CreateProjectResponse, ProjectDto, TranscriptDto};

#[command]
pub async fn create_project_cmd(
    title: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<ProjectDto, CommandError> {
    let req = CreateProjectRequest { title };
    let create_res = usecases
        .create_project
        .execute(req)
        .await
        .map_err(CommandError::from)?;

    Ok(ProjectDto::from(&create_res.project))
}

#[command]
pub async fn create_project_from_youtube_cmd(
    url: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<CreateProjectResponse, CommandError> {
    let req = CreateProjectFromYoutubeRequest { url };
    let response = usecases
        .create_project_from_youtube
        .execute(req)
        .await
        .map_err(CommandError::from)?;

    Ok(CreateProjectResponse {
        project: ProjectDto::from(&response.project),
        job: response.job,
    })
}

#[command]
pub async fn get_transcript_cmd(
    project_id: String,
    project_repo: State<'_, RuntimeProjectRepository>,
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
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<ProjectDto, CommandError> {
    let pid: domain::project::ProjectId = project_id
        .parse()
        .map_err(|e| CommandError::Validation(format!("Invalid project id: {}", e)))?;

    let req = GetProjectRequest { project_id: pid };
    let res = usecases
        .get_project
        .execute(req)
        .await
        .map_err(CommandError::from)?;
    Ok(ProjectDto::from(&res.project))
}

#[command]
pub async fn list_projects_cmd(
    project_repo: State<'_, RuntimeProjectRepository>,
) -> Result<Vec<ProjectDto>, CommandError> {
    let projects = project_repo
        .inner()
        .list()
        .await
        .map_err(|e| CommandError::Repository(e.to_string()))?;

    Ok(projects.into_iter().map(|p| ProjectDto::from(&p)).collect())
}

#[command]
pub async fn delete_project_cmd(
    project_id: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<(), CommandError> {
    let pid: domain::project::ProjectId = project_id
        .parse()
        .map_err(|e| CommandError::Validation(format!("Invalid project id: {}", e)))?;

    let req = DeleteProjectRequest { project_id: pid };
    usecases
        .delete_project
        .execute(req)
        .await
        .map_err(CommandError::from)?;
    Ok(())
}

#[command]
pub async fn start_project_mock_pipeline_cmd(
    project_id: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<CreateProjectResponse, CommandError> {
    let pid: domain::project::ProjectId = project_id
        .parse()
        .map_err(|e| CommandError::Validation(format!("Invalid project id: {}", e)))?;

    let req = StartMockPipelineRequest { project_id: pid };
    let response = usecases
        .start_mock_pipeline
        .execute(req)
        .await
        .map_err(CommandError::from)?;

    Ok(CreateProjectResponse {
        project: ProjectDto::from(&response.project),
        job: response.job,
    })
}
