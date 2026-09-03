use std::path::PathBuf;

use domain::media::{ArtifactKind, ArtifactLocation, MediaSource};
use domain::outbox::WorkspaceKey;
use domain::project::ProjectId;
use ports::source::{DownloadMediaRequest, VideoSourcePort};
use ports::storage::ArtifactStore;
use ports::transaction::CommitStagedArtifactWrite;

use crate::error::ApplicationError;

#[derive(Debug)]
pub struct DownloadYoutubeVideoRequest {
    pub project_id: ProjectId,
    pub source: MediaSource,
    pub temp_dir: PathBuf,
    pub workspace_key: WorkspaceKey,
    pub filename_hint: Option<String>,
}

pub struct DownloadYoutubeVideoUseCase<V, S> {
    video_source: V,
    artifact_store: S,
}

impl<V: VideoSourcePort, S: ArtifactStore> DownloadYoutubeVideoUseCase<V, S> {
    pub fn new(video_source: V, artifact_store: S) -> Self {
        Self {
            video_source,
            artifact_store,
        }
    }

    pub async fn execute(
        &self,
        request: DownloadYoutubeVideoRequest,
    ) -> Result<CommitStagedArtifactWrite, ApplicationError> {
        self.download(request, false).await
    }

    pub async fn execute_resumable(
        &self,
        request: DownloadYoutubeVideoRequest,
    ) -> Result<CommitStagedArtifactWrite, ApplicationError> {
        self.download(request, true).await
    }

    async fn download(
        &self,
        request: DownloadYoutubeVideoRequest,
        preserve_source: bool,
    ) -> Result<CommitStagedArtifactWrite, ApplicationError> {
        if !matches!(request.source, MediaSource::YoutubeUrl { .. }) {
            return Err(ApplicationError::InvalidOperation {
                message: "Source is not a YouTube URL".into(),
            });
        }
        let artifact = self
            .video_source
            .download_media(DownloadMediaRequest {
                source: request.source,
                target_dir: request.temp_dir,
                filename_hint: request.filename_hint.clone(),
            })
            .await?;
        let ArtifactLocation::LocalPath(path) = artifact.location else {
            return Err(ApplicationError::InvalidOperation {
                message: "Expected LocalPath from download_media".into(),
            });
        };
        let staged = if preserve_source {
            self.artifact_store
                .import_external_file(
                    &request.project_id,
                    ArtifactKind::DownloadedVideo,
                    std::path::Path::new(&path),
                    request.filename_hint.as_deref(),
                )
                .await?
        } else {
            self.artifact_store
                .stage_owned_temp_file(
                    &request.project_id,
                    ArtifactKind::DownloadedVideo,
                    std::path::Path::new(&path),
                    request.filename_hint.as_deref(),
                )
                .await?
        };
        Ok(CommitStagedArtifactWrite {
            project_id: request.project_id,
            artifact: staged.artifact,
            staging_key: staged.staging_key,
            final_key: staged.final_key,
            temp_workspace_key: Some(request.workspace_key),
        })
    }
}
