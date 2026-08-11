use crate::bootstrap::usecases::AppUseCases;
use crate::dto::error::{map_job_dto_result, parse_project_id, CommandError};
use crate::dto::project::{CreateProjectResponse, ProjectDto, SubtitleTrackDto, TranscriptDto};
use application::usecases::pipeline::start_mock::StartMockPipelineRequest;
use application::usecases::project::create::CreateProjectRequest;
use application::usecases::project::create_from_youtube::CreateProjectFromYoutubeRequest;
use application::usecases::project::delete::DeleteProjectRequest;
use application::usecases::project::get::GetProjectRequest;
use application::usecases::project::list::ListProjectsRequest;
use application::usecases::project::rename::RenameProjectRequest;
use application::usecases::transcript::get::GetTranscriptRequest;
use application::usecases::transcript::list_youtube_tracks::ListYoutubeSubtitleTracksRequest;

use std::sync::Arc;
use tauri::{command, AppHandle, State};

#[command]
pub async fn create_project_cmd(
    title: String,
    usecases: State<'_, Arc<AppUseCases>>,
    app_paths: State<'_, crate::bootstrap::paths::AppPaths>,
) -> Result<ProjectDto, CommandError> {
    let req = CreateProjectRequest { title };
    let create_res = usecases
        .create_project
        .execute(req)
        .await
        .map_err(CommandError::from)?;

    std::fs::create_dir_all(app_paths.project(create_res.project.id()))
        .map_err(|_| CommandError::Internal("Failed to create the project folder".to_string()))?;
    Ok(ProjectDto::from(&create_res.project))
}

#[command]
pub async fn rename_project_cmd(
    project_id: String,
    title: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<ProjectDto, CommandError> {
    let project = usecases
        .rename_project
        .execute(RenameProjectRequest {
            project_id: parse_project_id(&project_id)?,
            title,
        })
        .await
        .map_err(CommandError::from)?;
    Ok(ProjectDto::from(&project))
}

#[command]
pub async fn open_project_folder_cmd(
    project_id: String,
    app: AppHandle,
    app_paths: State<'_, crate::bootstrap::paths::AppPaths>,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<(), CommandError> {
    let project_id = parse_project_id(&project_id)?;
    usecases
        .get_project
        .execute(GetProjectRequest {
            project_id: project_id.clone(),
        })
        .await
        .map_err(CommandError::from)?;
    let path = app_paths.project(&project_id);
    std::fs::create_dir_all(&path)
        .map_err(|_| CommandError::Internal("Failed to create the project folder".to_string()))?;
    open_folder(&app, &path)?;
    Ok(())
}

#[allow(deprecated)]
fn open_folder(app: &AppHandle, path: &std::path::Path) -> Result<(), CommandError> {
    use tauri_plugin_shell::ShellExt;
    app.shell()
        .open(path.to_string_lossy().into_owned(), None)
        .map_err(|_| CommandError::Internal("Failed to open the project folder".to_string()))
}

#[command]
pub async fn create_project_from_youtube_cmd(
    url: String,
    project_id: Option<String>,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<ProjectDto, CommandError> {
    let req = CreateProjectFromYoutubeRequest {
        url,
        project_id: project_id.map(|id| parse_project_id(&id)).transpose()?,
    };
    let response = usecases
        .create_project_from_youtube
        .execute(req)
        .await
        .map_err(CommandError::from)?;

    Ok(ProjectDto::from(&response.project))
}

#[command]
pub async fn get_transcript_cmd(
    project_id: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<Option<TranscriptDto>, CommandError> {
    let pid = parse_project_id(&project_id)?;

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
pub async fn list_youtube_subtitle_tracks_cmd(
    project_id: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<Vec<SubtitleTrackDto>, CommandError> {
    let project_id = parse_project_id(&project_id)?;
    let response = usecases
        .list_youtube_subtitle_tracks
        .execute(ListYoutubeSubtitleTracksRequest { project_id })
        .await
        .map_err(CommandError::from)?;

    Ok(response.tracks.iter().map(SubtitleTrackDto::from).collect())
}

#[command]
pub async fn get_project_cmd(
    project_id: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<ProjectDto, CommandError> {
    let pid = parse_project_id(&project_id)?;

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
    let pid = parse_project_id(&project_id)?;

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
    subtitle_track_id: Option<String>,
    subtitle_language: Option<String>,
    subtitle_auto_generated: Option<bool>,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<CreateProjectResponse, CommandError> {
    let pid = parse_project_id(&project_id)?;

    let selected_subtitle_track = match (subtitle_track_id, subtitle_language) {
        (Some(id), Some(language)) => Some(domain::media::SubtitleTrack {
            id,
            language,
            label: None,
            format: Some("vtt".into()),
            is_auto_generated: subtitle_auto_generated.unwrap_or(false),
        }),
        _ => None,
    };
    let req = StartMockPipelineRequest {
        project_id: pid,
        selected_subtitle_track,
    };
    let response = usecases
        .start_mock_pipeline
        .execute(req)
        .await
        .map_err(CommandError::from)?;

    Ok(CreateProjectResponse {
        project: ProjectDto::from(&response.project),
        job: map_job_dto_result(adapters_tauri::dto::mapper::map_job_dto(&response.job))?,
    })
}
