use crate::bootstrap::usecases::AppUseCases;

use application::usecases::pipeline::start_mock::StartMockPipelineRequest;
use application::usecases::project::create::CreateProjectRequest;
use application::usecases::project::create_from_youtube::CreateProjectFromYoutubeRequest;
use application::usecases::project::delete::DeleteProjectRequest;
use application::usecases::project::get::GetProjectRequest;
use application::usecases::project::list::ListProjectsRequest;
use application::usecases::transcript::get::GetTranscriptRequest;

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
        job: crate::dto::job::JobDto::from(&response.job),
    })
}

#[command]
pub async fn get_transcript_cmd(
    project_id: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<Option<TranscriptDto>, CommandError> {
    let pid: domain::project::ProjectId = project_id
        .parse()
        .map_err(|e| CommandError::Validation(format!("Invalid project id: {}", e)))?;

    let req = GetTranscriptRequest { project_id: pid };
    let res = usecases
        .get_transcript
        .execute(req)
        .await
        .map_err(CommandError::from)?;

    if let Some(transcript) = res.transcript {
        Ok(Some(TranscriptDto::from(&transcript)))
    } else {
        Ok(None)
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
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<Vec<ProjectDto>, CommandError> {
    let req = ListProjectsRequest {};
    let res = usecases
        .list_projects
        .execute(req)
        .await
        .map_err(CommandError::from)?;

    Ok(res
        .projects
        .into_iter()
        .map(|p| ProjectDto::from(&p))
        .collect())
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
        job: crate::dto::job::JobDto::from(&response.job),
    })
}
