use super::usecase::{CreateProjectFromYoutubeUseCase, ensure_draft};
use crate::error::ApplicationError;
use domain::{
    media::MediaSource,
    project::{Project, ProjectId},
};
use ports::{
    repository::ProjectRepository,
    source::VideoSourcePort,
    storage::ArtifactStore,
    youtube_import::{YoutubeImportSession, YoutubeImportState},
};

impl<R: ProjectRepository, V: VideoSourcePort + Clone, S: ArtifactStore + Clone>
    CreateProjectFromYoutubeUseCase<R, V, S>
{
    pub(super) async fn prepare(
        &self,
        url: &str,
        project_id: Option<ProjectId>,
    ) -> Result<ProjectId, ApplicationError> {
        let mut project = match &project_id {
            Some(id) => self
                .project_repo
                .get(id)
                .await?
                .ok_or_else(|| ApplicationError::ProjectNotFound(id.clone()))?,
            None => Project::new(url.to_string())?,
        };
        ensure_draft(&project)?;
        let original_updated_at = project_id.as_ref().map(|_| project.updated_at());
        let source = MediaSource::YoutubeUrl { url: url.into() };
        self.video_source.validate_source(&source).await?;
        let metadata = self.video_source.fetch_metadata(&source).await?;
        project.import_source(source, Some(metadata))?;
        project.mark_ready_for_processing()?;
        let allocation = self
            .workspace_port
            .create_allocation(project.id(), "youtube-resume")
            .await?;
        let session = YoutubeImportSession {
            project: project.to_snapshot(),
            original_updated_at,
            workspace_key: allocation.workspace_key.clone(),
            write: None,
            state: YoutubeImportState::Downloading,
            revision: 1,
        };
        if let Err(error) = self.journal.insert(&session).await {
            return Err(super::super::youtube_cleanup::cleanup_failed_import(
                error.into(),
                None,
                &allocation.workspace_key,
                &self.artifact_store,
                self.workspace_port.as_ref(),
            )
            .await);
        }
        Ok(session.project.id)
    }
}
