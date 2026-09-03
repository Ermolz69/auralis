use crate::error::ApplicationError;
use crate::usecases::media::download_youtube_video::{
    DownloadYoutubeVideoRequest, DownloadYoutubeVideoUseCase,
};
use domain::media::MediaSource;
use domain::project::{Project, ProjectId, ProjectStatus};
use ports::error::PortError;
use ports::repository::ProjectRepository;
use ports::source::VideoSourcePort;
use ports::storage::ArtifactStore;
use ports::transaction::{CommitYoutubeImport, StorageUnitOfWork};
use ports::workspace::TempWorkspacePort;
use std::sync::Arc;

use super::lifecycle::ProjectLifecycleLocks;
use super::youtube_cleanup::cleanup_failed_import;

pub struct CreateProjectFromYoutubeRequest {
    pub url: String,
    pub project_id: Option<ProjectId>,
}

pub struct CreateProjectFromYoutubeResponse {
    pub project: Project,
}

pub struct CreateProjectFromYoutubeUseCase<R, V, S, T> {
    project_repo: R,
    video_source: V,
    artifact_store: S,
    storage_uow: T,
    workspace_port: Arc<dyn TempWorkspacePort>,
    locks: Arc<ProjectLifecycleLocks>,
}

impl<
    R: ProjectRepository,
    V: VideoSourcePort + Clone,
    S: ArtifactStore + Clone,
    T: StorageUnitOfWork,
> CreateProjectFromYoutubeUseCase<R, V, S, T>
{
    pub fn new(
        project_repo: R,
        video_source: V,
        artifact_store: S,
        storage_uow: T,
        workspace_port: Arc<dyn TempWorkspacePort>,
        locks: Arc<ProjectLifecycleLocks>,
    ) -> Self {
        Self {
            project_repo,
            video_source,
            artifact_store,
            storage_uow,
            workspace_port,
            locks,
        }
    }

    pub async fn execute(
        &self,
        request: CreateProjectFromYoutubeRequest,
    ) -> Result<CreateProjectFromYoutubeResponse, ApplicationError> {
        let mut project = match &request.project_id {
            Some(id) => self
                .project_repo
                .get(id)
                .await?
                .ok_or_else(|| ApplicationError::ProjectNotFound(id.clone()))?,
            None => Project::new(request.url.trim().into()),
        };
        if project.status() != &ProjectStatus::Draft || project.active_job_id().is_some() {
            return Err(ApplicationError::InvalidOperation {
                message: "YouTube import requires a Draft project without an active job".into(),
            });
        }
        let original_updated_at = request.project_id.as_ref().map(|_| project.updated_at());
        let source = MediaSource::YoutubeUrl {
            url: request.url.trim().into(),
        };
        self.video_source.validate_source(&source).await?;
        let metadata = self.video_source.fetch_metadata(&source).await?;
        project.import_source(source.clone(), Some(metadata))?;
        project.mark_ready_for_processing()?;

        let allocation = self
            .workspace_port
            .create_allocation(project.id(), "youtube-video-download")
            .await?;
        let download = DownloadYoutubeVideoUseCase::new(
            self.video_source.clone(),
            self.artifact_store.clone(),
        );
        let write = match download
            .execute(DownloadYoutubeVideoRequest {
                project_id: project.id().clone(),
                source,
                temp_dir: allocation.absolute_path,
                workspace_key: allocation.workspace_key.clone(),
                filename_hint: Some("original".into()),
            })
            .await
        {
            Ok(write) => write,
            Err(error) => {
                return Err(cleanup_failed_import(
                    error,
                    None,
                    &allocation.workspace_key,
                    &self.artifact_store,
                    self.workspace_port.as_ref(),
                )
                .await);
            }
        };
        let staging_key = write.staging_key.clone();
        let result = async {
            let lock = self.locks.get_lock(project.id())?;
            let _guard = lock.lock().await;
            if request.project_id.is_some() {
                let current = self
                    .project_repo
                    .get(project.id())
                    .await?
                    .ok_or_else(|| ApplicationError::ProjectNotFound(project.id().clone()))?;
                if current.revision() != project.revision()
                    || current.status() != &ProjectStatus::Draft
                    || current.active_job_id().is_some()
                {
                    return Err(ApplicationError::Port(PortError::Conflict {
                        resource: format!("Project {}", project.id()),
                        message: "Project changed during YouTube download; retry the import".into(),
                    }));
                }
            }
            let mut committed = project.clone();
            if original_updated_at.is_some() {
                committed.advance_revision()?;
            }
            self.storage_uow
                .commit_youtube_import(CommitYoutubeImport {
                    project,
                    write,
                    original_updated_at,
                })
                .await?;
            Ok(CreateProjectFromYoutubeResponse { project: committed })
        }
        .await;
        match result {
            Ok(response) => Ok(response),
            Err(error) => Err(cleanup_failed_import(
                error,
                Some(&staging_key),
                &allocation.workspace_key,
                &self.artifact_store,
                self.workspace_port.as_ref(),
            )
            .await),
        }
    }
}
