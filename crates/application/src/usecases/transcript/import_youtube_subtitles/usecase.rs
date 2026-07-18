use super::{
    cleanup::ImportCleanupCoordinator,
    vtt_parser::{parse_vtt, select_best_subtitle_track},
};
use crate::error::ApplicationError;
use domain::{project::ProjectId, transcript::Transcript};
use ports::{
    repository::ProjectRepository,
    source::{DownloadSubtitleRequest, SubtitleSourcePort},
    storage::ArtifactStore,
    transaction::{CommitTranscriptImport, StorageUnitOfWork},
    workspace::TempWorkspacePort,
};
use std::sync::Arc;

pub struct ImportYoutubeSubtitlesRequest {
    pub project_id: ProjectId,
    pub preferred_languages: Vec<String>,
    pub allow_auto_generated: bool,
    pub cancellation_token: tokio_util::sync::CancellationToken,
    pub job_id: domain::job::JobId,
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
        let project = self
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

        let best_track = select_best_subtitle_track(
            &subtitles,
            &request.preferred_languages,
            request.allow_auto_generated,
        )?;

        let alloc = self
            .workspace_port
            .create_allocation(&request.project_id, "subtitles")
            .await?;

        let download_request = DownloadSubtitleRequest {
            source: source.clone(),
            track: best_track.clone(),
            target_directory: alloc.absolute_path.clone(),
        };

        if request.cancellation_token.is_cancelled() {
            return Err(self
                .cleanup_coordinator
                .handle_workspace_failure(
                    &alloc.workspace_key,
                    ports::error::PortError::Cancelled.into(),
                )
                .await);
        }

        let download_fut = self.subtitle_source.download_subtitle(download_request);
        let artifact = tokio::select! {
            res = download_fut => {
                match res {
                    Ok(a) => a,
                    Err(e) => {
                        return Err(self.cleanup_coordinator
                            .handle_workspace_failure(&alloc.workspace_key, e.into())
                            .await);
                    }
                }
            }
            _ = request.cancellation_token.cancelled() => {
                return Err(self.cleanup_coordinator
                    .handle_workspace_failure(&alloc.workspace_key, ports::error::PortError::Cancelled.into())
                    .await);
            }
        };

        let vtt_path = match &artifact.location {
            domain::media::ArtifactLocation::LocalPath(p) => std::path::PathBuf::from(p),
            _ => {
                return Err(self
                    .cleanup_coordinator
                    .handle_workspace_failure(
                        &alloc.workspace_key,
                        ApplicationError::InvalidOperation {
                            message: "Invalid subtitle artifact location".to_string(),
                        },
                    )
                    .await);
            }
        };

        let filename = match vtt_path.file_name() {
            Some(name) => name.to_string_lossy().into_owned(),
            None => {
                return Err(self
                    .cleanup_coordinator
                    .handle_workspace_failure(
                        &alloc.workspace_key,
                        ApplicationError::InvalidOperation {
                            message: "Invalid subtitle filename".to_string(),
                        },
                    )
                    .await);
            }
        };

        if request.cancellation_token.is_cancelled() {
            return Err(self
                .cleanup_coordinator
                .handle_workspace_failure(
                    &alloc.workspace_key,
                    ports::error::PortError::Cancelled.into(),
                )
                .await);
        }

        let vtt_content = match self
            .workspace_port
            .read_workspace_file_to_string(&alloc.workspace_key, &filename, 10 * 1024 * 1024)
            .await
        {
            Ok(content) => content,
            Err(e) => {
                return Err(self
                    .cleanup_coordinator
                    .handle_workspace_failure(&alloc.workspace_key, e.into())
                    .await);
            }
        };

        let transcript = match parse_vtt(&vtt_content, &best_track.language) {
            Ok(t) => t,
            Err(e) => {
                return Err(self
                    .cleanup_coordinator
                    .handle_workspace_failure(&alloc.workspace_key, e)
                    .await);
            }
        };

        if request.cancellation_token.is_cancelled() {
            return Err(self
                .cleanup_coordinator
                .handle_workspace_failure(
                    &alloc.workspace_key,
                    ports::error::PortError::Cancelled.into(),
                )
                .await);
        }

        let staged = match self
            .artifact_store
            .stage_owned_workspace_file(
                &request.project_id,
                domain::media::ArtifactKind::OriginalSubtitle,
                self.workspace_port.as_ref(),
                &alloc.workspace_key,
                &filename,
                Some("subtitles.vtt"),
            )
            .await
        {
            Ok(s) => s,
            Err(e) => {
                return Err(self
                    .cleanup_coordinator
                    .handle_workspace_failure(&alloc.workspace_key, e.into())
                    .await);
            }
        };

        if request.cancellation_token.is_cancelled() {
            return Err(self
                .cleanup_coordinator
                .handle_all_failure(
                    &staged.staging_key,
                    &alloc.workspace_key,
                    ports::error::PortError::Cancelled.into(),
                )
                .await);
        }

        let mut current_project = match self.project_repo.get(&request.project_id).await {
            Ok(Some(p)) => p,
            Ok(None) => {
                let err = ApplicationError::ProjectNotFound(request.project_id.clone());
                return Err(self
                    .cleanup_coordinator
                    .handle_all_failure(&staged.staging_key, &alloc.workspace_key, err)
                    .await);
            }
            Err(e) => {
                return Err(self
                    .cleanup_coordinator
                    .handle_all_failure(&staged.staging_key, &alloc.workspace_key, e.into())
                    .await);
            }
        };

        let err_msg = if current_project.status() != &domain::project::ProjectStatus::Processing {
            Some("Project status is not Processing")
        } else if current_project.active_job_id() != Some(&request.job_id) {
            Some("Active job ID mismatch")
        } else if current_project.source() != Some(source) {
            Some("Project source changed")
        } else {
            None
        };
        if let Some(msg) = err_msg {
            let err = ApplicationError::InvalidOperation {
                message: msg.to_string(),
            };
            return Err(self
                .cleanup_coordinator
                .handle_all_failure(&staged.staging_key, &alloc.workspace_key, err)
                .await);
        }

        let expected_project_updated_at = current_project.updated_at();
        let expected_status = current_project.status().clone();
        let expected_active_job_id = current_project.active_job_id().cloned();

        current_project.set_transcript(transcript.clone());

        if let Err(e) = self
            .storage_uow
            .commit_transcript_import(CommitTranscriptImport {
                project: current_project,
                artifact: staged.artifact,
                staging_key: staged.staging_key.clone(),
                final_key: staged.final_key,
                temp_workspace_key: Some(alloc.workspace_key.clone()),
                expected_project_updated_at,
                expected_status,
                expected_active_job_id,
            })
            .await
        {
            return Err(self
                .cleanup_coordinator
                .handle_all_failure(&staged.staging_key, &alloc.workspace_key, e.into())
                .await);
        }

        Ok(ImportYoutubeSubtitlesResponse { transcript })
    }
}
