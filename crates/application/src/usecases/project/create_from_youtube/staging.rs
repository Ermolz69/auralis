use super::usecase::{CreateProjectFromYoutubeUseCase, conflict};
use crate::{
    error::ApplicationError,
    usecases::media::download_youtube_video::{
        DownloadYoutubeVideoRequest, DownloadYoutubeVideoUseCase,
    },
};
use domain::media::MediaSource;
use ports::{
    repository::ProjectRepository,
    source::VideoSourcePort,
    storage::ArtifactStore,
    youtube_import::{YoutubeImportSession, YoutubeImportState},
};

impl<R: ProjectRepository, V: VideoSourcePort + Clone, S: ArtifactStore + Clone>
    CreateProjectFromYoutubeUseCase<R, V, S>
{
    pub(super) async fn stage(
        &self,
        session: &mut YoutubeImportSession,
    ) -> Result<(), ApplicationError> {
        let source = MediaSource::YoutubeUrl {
            url: session.url().into(),
        };
        self.video_source.validate_source(&source).await?;
        if let Some(write) = &session.write {
            let size = write
                .artifact
                .size_bytes
                .ok_or_else(|| conflict("Missing staging size"))?;
            if !self
                .artifact_store
                .verify_staging(&write.staging_key, size)
                .await?
            {
                session.write = None;
                session.state = YoutubeImportState::Downloading;
                self.journal.checkpoint(session).await?;
            }
        }
        if session.write.is_none() {
            session.state = YoutubeImportState::Downloading;
            self.journal.checkpoint(session).await?;
            let temp_dir = self
                .workspace_port
                .resolve_key(&session.workspace_key)
                .await?;
            let downloader = DownloadYoutubeVideoUseCase::new(
                self.video_source.clone(),
                self.artifact_store.clone(),
            );
            let request = DownloadYoutubeVideoRequest {
                project_id: session.project.id.clone(),
                source: source.clone(),
                temp_dir,
                workspace_key: session.workspace_key.clone(),
                filename_hint: Some("original".into()),
            };
            let write = tokio::select! {
                result = downloader.execute_resumable(request) => result?,
                error = self.wait_for_discard(session) => return Err(error),
            };
            session.write = Some(write);
            session.state = YoutubeImportState::Staged;
            self.journal.checkpoint(session).await?;
        }
        Ok(())
    }

    async fn wait_for_discard(&self, session: &YoutubeImportSession) -> ApplicationError {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            match self.journal.find(&session.request_key()).await {
                Ok(Some(current))
                    if current.project.id == session.project.id
                        && current.revision == session.revision => {}
                Ok(_) => return conflict("Import was discarded or changed"),
                Err(error) => return error.into(),
            }
        }
    }
}
