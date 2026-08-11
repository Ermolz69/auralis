use crate::error::ApplicationError;
use crate::usecases::media::download_youtube_video::{
    DownloadYoutubeVideoRequest, DownloadYoutubeVideoUseCase,
};
use crate::usecases::project::create::{CreateProjectRequest, CreateProjectUseCase};
use crate::usecases::project::import_source::{ImportVideoSourceRequest, ImportVideoSourceUseCase};
use domain::media::MediaSource;
use domain::project::Project;
use ports::repository::ProjectRepository;
use ports::source::VideoSourcePort;
use ports::storage::ArtifactStore;
use ports::transaction::StorageUnitOfWork;
use ports::workspace::TempWorkspacePort;
use std::sync::Arc;

pub struct CreateProjectFromYoutubeRequest {
    pub url: String,
}

pub struct CreateProjectFromYoutubeResponse {
    pub project: Project,
}

pub struct CreateProjectFromYoutubeUseCase<
    R: ProjectRepository + Clone,
    V: VideoSourcePort + Clone,
    S: ArtifactStore + Clone,
    T: StorageUnitOfWork + Clone,
> {
    project_repo: R,
    video_source: V,
    artifact_store: S,
    storage_uow: T,
    workspace_port: Arc<dyn TempWorkspacePort>,
}

impl<
    R: ProjectRepository + Clone,
    V: VideoSourcePort + Clone,
    S: ArtifactStore + Clone,
    T: StorageUnitOfWork + Clone,
> CreateProjectFromYoutubeUseCase<R, V, S, T>
{
    pub fn new(
        project_repo: R,
        video_source: V,
        artifact_store: S,
        storage_uow: T,
        workspace_port: Arc<dyn TempWorkspacePort>,
    ) -> Self {
        Self {
            project_repo,
            video_source,
            artifact_store,
            storage_uow,
            workspace_port,
        }
    }

    pub async fn execute(
        &self,
        request: CreateProjectFromYoutubeRequest,
    ) -> Result<CreateProjectFromYoutubeResponse, ApplicationError> {
        let create_use_case = CreateProjectUseCase::new(self.project_repo.clone());
        let create_res = create_use_case
            .execute(CreateProjectRequest {
                title: request.url.clone(),
            })
            .await?;

        let import_use_case =
            ImportVideoSourceUseCase::new(self.project_repo.clone(), self.video_source.clone());
        let import_res = import_use_case
            .execute(ImportVideoSourceRequest {
                project_id: create_res.project.id().clone(),
                source: MediaSource::YoutubeUrl {
                    url: request.url.clone(),
                },
            })
            .await?;

        // Stage 1 owns only the source media and its metadata. Subtitle discovery/import is
        // deliberately started by the separate stage-2 command.
        let allocation = self
            .workspace_port
            .create_allocation(import_res.project.id(), "youtube-video-download")
            .await?;
        let download_use_case = DownloadYoutubeVideoUseCase::new(
            self.project_repo.clone(),
            self.video_source.clone(),
            self.artifact_store.clone(),
            self.storage_uow.clone(),
        );
        if let Err(error) = download_use_case
            .execute(DownloadYoutubeVideoRequest {
                project_id: import_res.project.id().clone(),
                temp_dir: allocation.absolute_path,
                filename_hint: Some("original".to_string()),
            })
            .await
        {
            let _ = self
                .workspace_port
                .delete_allocation(&allocation.workspace_key)
                .await;
            return Err(error);
        }

        let mut project = import_res.project;
        project.mark_ready_for_processing().map_err(|error| {
            ApplicationError::InvalidOperation {
                message: error.to_string(),
            }
        })?;
        self.project_repo.save(&project).await?;

        Ok(CreateProjectFromYoutubeResponse { project })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{MockArtifactStore, MockStorageUnitOfWork};
    use adapters_storage::{local::LocalTempWorkspace, memory::InMemoryProjectRepository};
    use adapters_ytdlp::mock::MockVideoSourceAdapter;
    use domain::project::ProjectStatus;

    #[tokio::test]
    async fn stage_one_downloads_media_and_stops_before_subtitles() {
        let repo = InMemoryProjectRepository::new(Arc::new(std::sync::Mutex::new(
            adapters_storage::memory::InMemoryDatabase::new(),
        )));
        let temp_root = tempfile::tempdir().unwrap();
        let use_case = CreateProjectFromYoutubeUseCase::new(
            repo.clone(),
            MockVideoSourceAdapter::new(),
            MockArtifactStore,
            MockStorageUnitOfWork::new(),
            Arc::new(LocalTempWorkspace::new(temp_root.path().to_path_buf())),
        );

        let response = use_case
            .execute(CreateProjectFromYoutubeRequest {
                url: "https://www.youtube.com/watch?v=stage-one".into(),
            })
            .await
            .unwrap();

        assert_eq!(
            response.project.status(),
            &ProjectStatus::ReadyForProcessing
        );
        assert!(response.project.metadata().is_some());
        assert!(response.project.transcript().is_none());
        assert_eq!(
            repo.get(response.project.id())
                .await
                .unwrap()
                .unwrap()
                .status(),
            &ProjectStatus::ReadyForProcessing
        );
    }
}
