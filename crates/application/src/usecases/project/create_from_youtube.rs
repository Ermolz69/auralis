use crate::error::ApplicationError;
use crate::usecases::pipeline::start_mock::{StartMockPipelineRequest, StartMockPipelineUseCase};
use crate::usecases::project::create::{CreateProjectRequest, CreateProjectUseCase};
use crate::usecases::project::import_source::{ImportVideoSourceRequest, ImportVideoSourceUseCase};
use domain::media::MediaSource;
use domain::project::Project;
use ports::artifact_index::ArtifactIndex;
use ports::job_scheduler::{JobSchedulerPort, ScheduledJob};
use ports::repository::ProjectRepository;
use ports::source::{SubtitleSourcePort, VideoSourcePort};
use ports::storage::ArtifactStore;
use ports::transaction::StorageUnitOfWork;
use std::sync::Arc;

pub struct CreateProjectFromYoutubeRequest {
    pub url: String,
}

pub struct CreateProjectFromYoutubeResponse {
    pub project: Project,
    pub job: ScheduledJob,
}

pub struct CreateProjectFromYoutubeUseCase<
    R: ProjectRepository + Clone + 'static,
    V: VideoSourcePort + Clone,
    SSub: SubtitleSourcePort + Clone + 'static,
    SStore: ArtifactStore + Clone + 'static,
> {
    project_repo: R,
    video_source: V,
    job_scheduler: Arc<dyn JobSchedulerPort>,
    storage_uow: Arc<dyn StorageUnitOfWork>,
    subtitle_source: SSub,
    artifact_store: SStore,
    target_dir_base: std::path::PathBuf,
}

impl<
    R: ProjectRepository + Clone + 'static,
    V: VideoSourcePort + Clone,
    SSub: SubtitleSourcePort + Clone + 'static,
    SStore: ArtifactStore + Clone + 'static,
> CreateProjectFromYoutubeUseCase<R, V, SSub,  SStore>
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_repo: R,
        video_source: V,
        job_scheduler: Arc<dyn JobSchedulerPort>,
        storage_uow: Arc<dyn StorageUnitOfWork>,
        subtitle_source: SSub,
            artifact_store: SStore,
        target_dir_base: std::path::PathBuf,
    ) -> Self {
        Self {
            project_repo,
            video_source,
            job_scheduler,
            storage_uow,
            subtitle_source,
                        artifact_store,
            target_dir_base,
        }
    }

    pub async fn execute(
        &self,
        request: CreateProjectFromYoutubeRequest,
    ) -> Result<CreateProjectFromYoutubeResponse, ApplicationError> {
        let create_use_case = CreateProjectUseCase::new(self.project_repo.clone());
        let req1 = CreateProjectRequest {
            title: request.url.clone(),
        };
        let create_res = create_use_case.execute(req1).await?;

        let import_use_case =
            ImportVideoSourceUseCase::new(self.project_repo.clone(), self.video_source.clone());
        let req2 = ImportVideoSourceRequest {
            project_id: create_res.project.id().clone(),
            source: MediaSource::YoutubeUrl {
                url: request.url.clone(),
            },
        };
        let import_res = import_use_case.execute(req2).await?;

        let mut proj = import_res.project;
        proj.mark_ready_for_processing()
            .map_err(|e| ApplicationError::InvalidOperation {
                message: e.to_string(),
            })?;
        self.project_repo.save(&proj).await?;

        let pipeline_use_case = StartMockPipelineUseCase::new(
            self.project_repo.clone(),
            self.job_scheduler.clone(),
            self.storage_uow.clone(),
            self.subtitle_source.clone(),
                        self.artifact_store.clone(),
            self.target_dir_base.clone(),
        );
        let req3 = StartMockPipelineRequest {
            project_id: proj.id().clone(),
        };
        let pipeline_res = pipeline_use_case.execute(req3).await?;

        Ok(CreateProjectFromYoutubeResponse {
            project: pipeline_res.project,
            job: pipeline_res.job,
        })
    }
}
