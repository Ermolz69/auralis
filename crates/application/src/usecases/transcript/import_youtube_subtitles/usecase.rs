use std::sync::Arc;

use domain::project::ProjectId;
use domain::transcript::Transcript;
use ports::repository::ProjectRepository;
use ports::source::{DownloadSubtitleRequest, SubtitleSourcePort};
use ports::storage::ArtifactStore;
use ports::transaction::{CommitTranscriptImport, StorageUnitOfWork};
use ports::workspace::TempWorkspacePort;

use super::cleanup::ImportCleanupCoordinator;
use super::vtt_parser::parse_vtt;
use crate::error::ApplicationError;

pub struct ImportYoutubeSubtitlesRequest {
    pub project_id: ProjectId,
    pub preferred_languages: Vec<String>,
    pub allow_auto_generated: bool,
}

pub struct ImportYoutubeSubtitlesResponse {
    pub transcript: Transcript,
}

pub struct ImportYoutubeSubtitlesUseCase {
    project_repo: Arc<dyn ProjectRepository>,
    subtitle_source: Arc<dyn SubtitleSourcePort>,
    artifact_store: Arc<dyn ArtifactStore>,
    storage_uow: Arc<dyn StorageUnitOfWork>,
    workspace_port: Arc<dyn TempWorkspacePort>,
    cleanup_coordinator: ImportCleanupCoordinator,
}

impl ImportYoutubeSubtitlesUseCase {
    pub fn new(
        project_repo: Arc<dyn ProjectRepository>,
        subtitle_source: Arc<dyn SubtitleSourcePort>,
        artifact_store: Arc<dyn ArtifactStore>,
        storage_uow: Arc<dyn StorageUnitOfWork>,
        workspace_port: Arc<dyn TempWorkspacePort>,
    ) -> Self {
        Self {
            project_repo,
            subtitle_source,
            artifact_store: artifact_store.clone(),
            storage_uow,
            workspace_port: workspace_port.clone(),
            cleanup_coordinator: ImportCleanupCoordinator::new(artifact_store, workspace_port),
        }
    }

    pub async fn execute(
        &self,
        request: ImportYoutubeSubtitlesRequest,
    ) -> Result<ImportYoutubeSubtitlesResponse, ApplicationError> {
        let mut project = self
            .project_repo
            .get(&request.project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(request.project_id.clone()))?;

        let source = project
            .source()
            .ok_or_else(|| ApplicationError::InvalidOperation {
                message: "Project has no source".to_string(),
            })?;

        let subtitles = self.subtitle_source.list_subtitles(source).await?;
        if subtitles.is_empty() {
            return Err(ApplicationError::InvalidOperation {
                message: "No subtitles found".to_string(),
            });
        }

        let is_vtt = |t: &domain::media::SubtitleTrack| t.format.as_deref() == Some("vtt");

        // Pick best subtitle track
        let mut best_track = None;
        for lang in &request.preferred_languages {
            if let Some(track) = subtitles
                .iter()
                .find(|t| &t.language == lang && !t.is_auto_generated && is_vtt(t))
            {
                best_track = Some(track);
                break;
            }
        }

        if best_track.is_none() && request.allow_auto_generated {
            for lang in &request.preferred_languages {
                if let Some(track) = subtitles
                    .iter()
                    .find(|t| &t.language == lang && t.is_auto_generated && is_vtt(t))
                {
                    best_track = Some(track);
                    break;
                }
            }
        }

        if best_track.is_none() {
            best_track = subtitles.iter().find(|t| !t.is_auto_generated && is_vtt(t));
            if best_track.is_none() && request.allow_auto_generated {
                best_track = subtitles.iter().find(|t| is_vtt(t));
            }
        }

        let best_track = best_track.ok_or_else(|| ApplicationError::InvalidOperation {
            message: "No suitable subtitles found".to_string(),
        })?;

        let alloc = self
            .workspace_port
            .create_allocation(&request.project_id, "subtitles")
            .await?;

        let download_request = DownloadSubtitleRequest {
            source: source.clone(),
            track: best_track.clone(),
            target_directory: alloc.absolute_path.clone(),
        };

        let artifact = match self
            .subtitle_source
            .download_subtitle(download_request)
            .await
        {
            Ok(a) => a,
            Err(e) => {
                self.cleanup_coordinator
                    .cleanup_workspace(&alloc.workspace_key)
                    .await;
                return Err(e.into());
            }
        };

        let vtt_path = match &artifact.location {
            domain::media::ArtifactLocation::LocalPath(p) => std::path::PathBuf::from(p),
            _ => {
                self.cleanup_coordinator
                    .cleanup_workspace(&alloc.workspace_key)
                    .await;
                return Err(ApplicationError::InvalidOperation {
                    message: "Invalid subtitle artifact location".to_string(),
                });
            }
        };

        let vtt_content = match tokio::fs::read_to_string(&vtt_path).await {
            Ok(content) => content,
            Err(e) => {
                self.cleanup_coordinator
                    .cleanup_workspace(&alloc.workspace_key)
                    .await;
                return Err(ApplicationError::InvalidOperation {
                    message: format!("Failed to read vtt file: {}", e),
                });
            }
        };

        let transcript = match parse_vtt(&vtt_content, &best_track.language) {
            Ok(t) => t,
            Err(e) => {
                self.cleanup_coordinator
                    .cleanup_workspace(&alloc.workspace_key)
                    .await;
                return Err(e);
            }
        };

        let staged = match self
            .artifact_store
            .stage_owned_temp_file(
                &request.project_id,
                domain::media::ArtifactKind::OriginalSubtitle,
                &vtt_path,
                Some("subtitles.vtt"),
            )
            .await
        {
            Ok(s) => s,
            Err(e) => {
                self.cleanup_coordinator
                    .cleanup_workspace(&alloc.workspace_key)
                    .await;
                return Err(e.into());
            }
        };

        project.set_transcript(transcript.clone());

        if let Err(e) = self
            .storage_uow
            .commit_transcript_import(CommitTranscriptImport {
                project,
                artifact: staged.artifact,
                staging_key: staged.staging_key.clone(),
                final_key: staged.final_key,
                temp_workspace_key: Some(alloc.workspace_key.clone()),
            })
            .await
        {
            self.cleanup_coordinator
                .cleanup_all(&staged.staging_key, &alloc.workspace_key)
                .await;
            return Err(e.into());
        }

        Ok(ImportYoutubeSubtitlesResponse { transcript })
    }
}
