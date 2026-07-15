use std::path::PathBuf;

use domain::media::ArtifactKind;
use domain::project::ProjectId;
use ports::repository::ProjectRepository;
use ports::source::{DownloadMediaRequest, VideoSourcePort};
use ports::storage::ArtifactStore;
use ports::transaction::{CommitStagedArtifactWrite, StorageUnitOfWork};

use crate::error::ApplicationError;

#[derive(Debug)]
pub struct DownloadYoutubeVideoRequest {
    pub project_id: ProjectId,
    pub temp_dir: PathBuf,
    pub filename_hint: Option<String>,
}

pub struct DownloadYoutubeVideoUseCase<P, V, S, T>
where
    P: ProjectRepository,
    V: VideoSourcePort,
    S: ArtifactStore,
    T: StorageUnitOfWork,
{
    project_repo: P,
    video_source: V,
    artifact_store: S,
    storage_uow: T,
}

impl<P, V, S, T> DownloadYoutubeVideoUseCase<P, V, S, T>
where
    P: ProjectRepository,
    V: VideoSourcePort,
    S: ArtifactStore,
    T: StorageUnitOfWork,
{
    pub fn new(project_repo: P, video_source: V, artifact_store: S, storage_uow: T) -> Self {
        Self {
            project_repo,
            video_source,
            artifact_store,
            storage_uow,
        }
    }

    pub async fn execute(
        &self,
        request: DownloadYoutubeVideoRequest,
    ) -> Result<(), ApplicationError> {
        let project = self
            .project_repo
            .get(&request.project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(request.project_id.clone()))?;

        let source = project
            .source()
            .ok_or_else(|| ApplicationError::InvalidOperation {
                message: "Project has no media source".into(),
            })?;

        if !matches!(
            source,
            domain::media::MediaSource::YoutubeUrl { .. }
                | domain::media::MediaSource::RemoteUrl { .. }
        ) {
            return Err(ApplicationError::InvalidOperation {
                message: "Source is not a remote URL or YouTube URL".into(),
            });
        }

        std::fs::create_dir_all(&request.temp_dir).map_err(|e| {
            ApplicationError::InvalidOperation {
                message: format!("Failed to create temp directory: {}", e),
            }
        })?;

        let download_req = DownloadMediaRequest {
            source: source.clone(),
            target_dir: request.temp_dir.clone(),
            filename_hint: request.filename_hint.clone(),
        };

        // 1. Download to temporary path
        let temp_artifact = self.video_source.download_media(download_req).await?;

        let temp_path = match temp_artifact.location {
            domain::media::ArtifactLocation::LocalPath(p) => std::path::PathBuf::from(p),
            domain::media::ArtifactLocation::StorageKey(_) => {
                return Err(ApplicationError::InvalidOperation {
                    message: "Expected LocalPath from download_media, got StorageKey".into(),
                });
            }
        };

        // 2. Stage artifact in the ArtifactStore
        let staged = match self
            .artifact_store
            .stage_owned_temp_file(
                &request.project_id,
                ArtifactKind::DownloadedVideo,
                &temp_path,
                request.filename_hint.as_deref(),
            )
            .await
        {
            Ok(s) => s,
            Err(e) => {
                let _ = std::fs::remove_file(&temp_path);
                return Err(ApplicationError::Port(e));
            }
        };

        let temp_workspace_key = temp_path
            .strip_prefix(&request.temp_dir)
            .ok()
            .and_then(|p| p.to_str())
            .and_then(|s| domain::outbox::WorkspaceKey::new(s.replace('\\', "/")).ok());

        // 3. Atomically persist to DB and write outbox message
        let commit_cmd = CommitStagedArtifactWrite {
            project_id: request.project_id.clone(),
            artifact: staged.artifact,
            staging_key: staged.staging_key.clone(),
            final_key: staged.final_key.clone(),
            temp_workspace_key,
        };

        if let Err(e) = self
            .storage_uow
            .commit_staged_artifact_write(commit_cmd)
            .await
        {
            // DB failed, we can optionally clean up the staging file
            let _ = self
                .artifact_store
                .delete_storage_key(&staged.staging_key)
                .await;
            return Err(ApplicationError::Port(e));
        }

        Ok(())
    }
}
