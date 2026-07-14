use std::sync::Arc;
use tauri::AppHandle;
use tauri::Manager;

use crate::state::{
    RuntimeArtifactIndex, RuntimeArtifactStore, RuntimeProjectRepository, RuntimeStorageUnitOfWork,
};
use adapters_ffmpeg::ffprobe::FfprobeAdapter;
use adapters_ytdlp::ytdlp::YtDlpAdapter;
use application::usecases::artifact::list_project_artifacts::ListProjectArtifactsUseCase;
use application::usecases::artifact::resolve_path::ResolveArtifactPathUseCase;
use application::usecases::job::cancel::CancelJobUseCase;
use application::usecases::job::list::ListJobsUseCase;
use application::usecases::media::import_local_media::ImportLocalMediaUseCase;
use application::usecases::media::probe_local::ProbeLocalMediaUseCase;
use application::usecases::pipeline::start_mock::StartMockPipelineUseCase;
use application::usecases::project::create::CreateProjectUseCase;
use application::usecases::project::create_from_youtube::CreateProjectFromYoutubeUseCase;
use application::usecases::project::delete::DeleteProjectUseCase;
use application::usecases::project::get::GetProjectUseCase;
use application::usecases::project::list::ListProjectsUseCase;
use application::usecases::transcript::get::GetTranscriptUseCase;
use ports::job_scheduler::JobSchedulerPort;

pub struct AppUseCases {
    pub list_project_artifacts: ListProjectArtifactsUseCase<RuntimeArtifactIndex>,
    pub resolve_artifact_path:
        ResolveArtifactPathUseCase<RuntimeArtifactIndex, RuntimeArtifactStore>,
    pub probe_local_media: ProbeLocalMediaUseCase<FfprobeAdapter>,
    pub import_local_media: ImportLocalMediaUseCase<
        RuntimeProjectRepository,
        FfprobeAdapter,
        YtDlpAdapter,
        RuntimeArtifactStore,
    >,
    pub create_project: CreateProjectUseCase<RuntimeProjectRepository>,
    pub create_project_from_youtube: CreateProjectFromYoutubeUseCase<
        RuntimeProjectRepository,
        YtDlpAdapter,
        YtDlpAdapter,
        RuntimeArtifactStore,
    >,
    pub get_project: GetProjectUseCase<RuntimeProjectRepository>,
    pub list_projects: ListProjectsUseCase<RuntimeProjectRepository>,
    pub delete_project: DeleteProjectUseCase,
    pub start_mock_pipeline:
        StartMockPipelineUseCase<RuntimeProjectRepository, YtDlpAdapter, RuntimeArtifactStore>,
    pub get_transcript: GetTranscriptUseCase<RuntimeProjectRepository>,
    pub list_jobs: ListJobsUseCase,
    pub cancel_job: CancelJobUseCase,
}

pub fn setup_usecases(
    app: &AppHandle,
    project_repo: RuntimeProjectRepository,
    artifact_index: RuntimeArtifactIndex,
    artifact_store: RuntimeArtifactStore,
    storage_uow: RuntimeStorageUnitOfWork,
    job_scheduler: Arc<dyn JobSchedulerPort>,
    workspace_root: std::path::PathBuf,
) {
    let ytdlp_candidates = crate::bootstrap::media_tools::resolve_ytdlp_candidates(app);
    let ytdlp_adapter = YtDlpAdapter::new(ytdlp_candidates);

    let probe = FfprobeAdapter::new(crate::bootstrap::media_tools::resolve_ffprobe_candidates(
        app,
    ));

    let usecases = AppUseCases {
        list_project_artifacts: ListProjectArtifactsUseCase::new(artifact_index.clone()),
        resolve_artifact_path: ResolveArtifactPathUseCase::new(
            artifact_index.clone(),
            artifact_store.clone(),
        ),
        probe_local_media: ProbeLocalMediaUseCase::new(probe.clone()),
        import_local_media: ImportLocalMediaUseCase::new(
            project_repo.clone(),
            probe.clone(),
            job_scheduler.clone(),
            storage_uow.clone(),
            ytdlp_adapter.clone(),
            artifact_store.clone(),
            artifact_index.clone(),
            workspace_root.clone(),
        ),
        create_project: CreateProjectUseCase::new(project_repo.clone()),
        create_project_from_youtube: CreateProjectFromYoutubeUseCase::new(
            project_repo.clone(),
            ytdlp_adapter.clone(),
            job_scheduler.clone(),
            storage_uow.clone(),
            ytdlp_adapter.clone(),
            artifact_store.clone(),
            workspace_root.clone(),
        ),
        get_project: GetProjectUseCase::new(project_repo.clone()),
        list_projects: ListProjectsUseCase::new(project_repo.clone()),
        delete_project: DeleteProjectUseCase::new(artifact_index.clone(), storage_uow.clone()),
        start_mock_pipeline: StartMockPipelineUseCase::new(
            project_repo.clone(),
            job_scheduler.clone(),
            storage_uow.clone(),
            ytdlp_adapter.clone(),
            artifact_store.clone(),
            workspace_root.clone(),
        ),
        get_transcript: GetTranscriptUseCase::new(project_repo.clone()),
        list_jobs: ListJobsUseCase::new(job_scheduler.clone()),
        cancel_job: CancelJobUseCase::new(job_scheduler.clone()),
    };

    app.manage(Arc::new(usecases));
}
