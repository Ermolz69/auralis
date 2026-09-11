use crate::bootstrap::usecases::AppUseCases;
use crate::dto::error::{CommandError, map_job_dto_result, parse_project_id};
use crate::dto::project::{CreateProjectResponse, ProjectDto, SubtitleTrackDto, TranscriptDto};
use application::usecases::pipeline::start_mock::StartMockPipelineRequest;
use application::usecases::project::create::CreateProjectRequest;
use application::usecases::project::create_from_youtube::CreateProjectFromYoutubeRequest;
use application::usecases::project::delete::DeleteProjectRequest;
use application::usecases::project::get::GetProjectRequest;
use application::usecases::project::list::ListProjectsRequest;
use application::usecases::project::open_folder::OpenProjectFolderRequest;
use application::usecases::project::rename::RenameProjectRequest;
use application::usecases::transcript::get::GetTranscriptRequest;
use application::usecases::transcript::list_youtube_tracks::ListYoutubeSubtitleTracksRequest;

use std::sync::Arc;
use tauri::{State, command};

#[command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn create_project_cmd(
    request: tauri::ipc::Request<'_>,
    title: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<ProjectDto, CommandError> {
    crate::observability::command::observe("create_project_cmd", async {
        let req = CreateProjectRequest { title };
        let create_res = usecases
            .create_project
            .execute(req)
            .await
            .map_err(CommandError::from)?;

        Ok(ProjectDto::from(&create_res.project))
    })
    .await
}

#[command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn rename_project_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    title: String,
    expected_revision: u64,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<ProjectDto, CommandError> {
    crate::observability::command::observe("rename_project_cmd", async {
        let project = usecases
            .rename_project
            .execute(RenameProjectRequest {
                expected_revision,
                project_id: parse_project_id(&project_id)?,
                title,
            })
            .await
            .map_err(CommandError::from)?;
        Ok(ProjectDto::from(&project))
    })
    .await
}

#[command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn open_project_folder_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<(), CommandError> {
    crate::observability::command::observe("open_project_folder_cmd", async {
        usecases
            .open_project_folder
            .execute(OpenProjectFolderRequest {
                project_id: parse_project_id(&project_id)?,
            })
            .await
            .map_err(CommandError::from)?;
        Ok(())
    })
    .await
}

#[command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn create_project_from_youtube_cmd(
    request: tauri::ipc::Request<'_>,
    url: String,
    project_id: Option<String>,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<ProjectDto, CommandError> {
    crate::observability::command::observe("create_project_from_youtube_cmd", async {
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
    })
    .await
}

#[command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn get_transcript_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<Option<TranscriptDto>, CommandError> {
    crate::observability::command::observe("get_transcript_cmd", async {
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
    })
    .await
}

#[command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn list_youtube_subtitle_tracks_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<Vec<SubtitleTrackDto>, CommandError> {
    crate::observability::command::observe("list_youtube_subtitle_tracks_cmd", async {
        let project_id = parse_project_id(&project_id)?;
        let response = usecases
            .list_youtube_subtitle_tracks
            .execute(ListYoutubeSubtitleTracksRequest { project_id })
            .await
            .map_err(CommandError::from)?;

        Ok(response.tracks.iter().map(SubtitleTrackDto::from).collect())
    })
    .await
}

#[command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn get_project_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<ProjectDto, CommandError> {
    crate::observability::command::observe("get_project_cmd", async {
        let pid = parse_project_id(&project_id)?;

        let req = GetProjectRequest { project_id: pid };
        let res = usecases
            .get_project
            .execute(req)
            .await
            .map_err(CommandError::from)?;
        Ok(ProjectDto::from(&res.project))
    })
    .await
}

#[command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn list_projects_cmd(
    request: tauri::ipc::Request<'_>,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<Vec<ProjectDto>, CommandError> {
    crate::observability::command::observe("list_projects_cmd", async {
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
    })
    .await
}

#[command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn delete_project_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<(), CommandError> {
    crate::observability::command::observe("delete_project_cmd", async {
        let pid = parse_project_id(&project_id)?;

        let req = DeleteProjectRequest { project_id: pid };
        usecases
            .delete_project
            .execute(req)
            .await
            .map_err(CommandError::from)?;
        Ok(())
    })
    .await
}

#[command]
#[tracing::instrument(skip_all, fields(request_id = %crate::observability::request::request_id(&request)))]
pub async fn start_project_mock_pipeline_cmd(
    request: tauri::ipc::Request<'_>,
    project_id: String,
    subtitle_track_id: Option<String>,
    subtitle_language: Option<String>,
    subtitle_auto_generated: Option<bool>,
    usecases: State<'_, Arc<AppUseCases>>,
) -> Result<CreateProjectResponse, CommandError> {
    crate::observability::command::observe("start_project_mock_pipeline_cmd", async {
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
    })
    .await
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    #[test]
    fn create_project_command_does_not_manage_project_directories() {
        let source = include_str!("project.rs");
        let create_command = source
            .split("pub async fn create_project_cmd")
            .nth(1)
            .and_then(|source| source.split("pub async fn rename_project_cmd").next())
            .expect("create project command source");

        assert!(!create_command.contains("AppPaths"));
        assert!(!create_command.contains("create_dir_all"));
    }

    #[test]
    fn open_project_folder_command_delegates_without_accepting_a_path() {
        let source = include_str!("project.rs");
        let open_folder_command = source
            .split("pub async fn open_project_folder_cmd")
            .nth(1)
            .and_then(|source| {
                source
                    .split("pub async fn create_project_from_youtube_cmd")
                    .next()
            })
            .expect("open project folder command source");

        assert!(open_folder_command.contains("open_project_folder"));
        assert!(open_folder_command.contains("parse_project_id(&project_id)"));
        assert!(!open_folder_command.contains("AppPaths"));
        assert!(!open_folder_command.contains("create_dir_all"));
        assert!(!open_folder_command.contains("path: String"));
    }

    #[test]
    fn delete_project_command_does_not_delete_project_files_directly() {
        let source = include_str!("project.rs");
        let delete_command = source
            .split("pub async fn delete_project_cmd")
            .nth(1)
            .and_then(|source| {
                source
                    .split("pub async fn start_project_mock_pipeline_cmd")
                    .next()
            })
            .expect("delete project command source");

        assert!(!delete_command.contains("AppPaths"));
        assert!(!delete_command.contains("remove_dir"));
        assert!(!delete_command.contains("delete_project_dir"));
    }
}
