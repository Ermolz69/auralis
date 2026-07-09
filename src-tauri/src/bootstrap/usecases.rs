use std::sync::Arc;
use tauri::AppHandle;
use tauri::Manager;

use crate::state::{
    RuntimeArtifactIndex, RuntimeArtifactStore, RuntimeProjectRepository, RuntimeTransactionGateway,
};
use adapters_ffmpeg::ffprobe::FfprobeAdapter;
use adapters_ytdlp::ytdlp::YtDlpAdapter;
use application::usecases::artifact::list_project_artifacts::ListProjectArtifactsUseCase;
use application::usecases::media::import_local_media::ImportLocalMediaUseCase;
use application::usecases::media::probe_local::ProbeLocalMediaUseCase;
use application::usecases::pipeline::start_mock::StartMockPipelineUseCase;
use application::usecases::project::create::CreateProjectUseCase;
use application::usecases::project::create_from_youtube::CreateProjectFromYoutubeUseCase;
use application::usecases::project::delete::DeleteProjectUseCase;
use application::usecases::project::get::GetProjectUseCase;
use ports::job_scheduler::JobSchedulerPort;

pub struct AppUseCases {
    pub list_project_artifacts: ListProjectArtifactsUseCase<RuntimeArtifactIndex>,
    pub probe_local_media: ProbeLocalMediaUseCase<RuntimeProjectRepository, FfprobeAdapter>,
    pub import_local_media: ImportLocalMediaUseCase<
        RuntimeProjectRepository,
        FfprobeAdapter,
        YtDlpAdapter,
        RuntimeArtifactIndex,
        RuntimeArtifactStore,
    >,
    pub create_project: CreateProjectUseCase<RuntimeProjectRepository>,
    pub create_project_from_youtube: CreateProjectFromYoutubeUseCase<
        RuntimeProjectRepository,
        YtDlpAdapter,
        YtDlpAdapter,
        RuntimeArtifactIndex,
        RuntimeArtifactStore,
    >,
    pub get_project: GetProjectUseCase<RuntimeProjectRepository>,
    pub delete_project: DeleteProjectUseCase,
    pub start_mock_pipeline: StartMockPipelineUseCase<
        RuntimeProjectRepository,
        YtDlpAdapter,
        RuntimeArtifactIndex,
        RuntimeArtifactStore,
    >,
}

pub fn setup_usecases(
    app: &AppHandle,
    project_repo: RuntimeProjectRepository,
    artifact_index: RuntimeArtifactIndex,
    artifact_store: RuntimeArtifactStore,
    transaction_gateway: RuntimeTransactionGateway,
    job_scheduler: Arc<dyn JobSchedulerPort>,
) {
    let ytdlp_candidates = crate::bootstrap::media_tools::resolve_ytdlp_candidates(app);
    let ytdlp_adapter = YtDlpAdapter::new(ytdlp_candidates);
    let target_dir_base = app
        .path()
        .app_data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));

    let probe = FfprobeAdapter::new(crate::bootstrap::media_tools::resolve_ffprobe_candidates(
        app,
    ));

    let usecases = AppUseCases {
        list_project_artifacts: ListProjectArtifactsUseCase::new(artifact_index.clone()),
        probe_local_media: ProbeLocalMediaUseCase::new(project_repo.clone(), probe.clone()),
        import_local_media: ImportLocalMediaUseCase::new(
            project_repo.clone(),
            probe.clone(),
            job_scheduler.clone(),
            transaction_gateway.clone(),
            ytdlp_adapter.clone(),
            artifact_index.clone(),
            artifact_store.clone(),
            target_dir_base.clone(),
        ),
        create_project: CreateProjectUseCase::new(project_repo.clone()),
        create_project_from_youtube: CreateProjectFromYoutubeUseCase::new(
            project_repo.clone(),
            ytdlp_adapter.clone(),
            job_scheduler.clone(),
            transaction_gateway.clone(),
            ytdlp_adapter.clone(),
            artifact_index.clone(),
            artifact_store.clone(),
            target_dir_base.clone(),
        ),
        get_project: GetProjectUseCase::new(project_repo.clone()),
        delete_project: DeleteProjectUseCase::new(
            artifact_index.clone(),
            transaction_gateway.clone(),
        ),
        start_mock_pipeline: StartMockPipelineUseCase::new(
            project_repo.clone(),
            job_scheduler.clone(),
            transaction_gateway.clone(),
            ytdlp_adapter.clone(),
            artifact_index.clone(),
            artifact_store.clone(),
            target_dir_base.clone(),
        ),
    };

    app.manage(Arc::new(usecases));
}
