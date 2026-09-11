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
use application::usecases::pipeline::start_mock::{
    StartMockPipelineDependencies, StartMockPipelineUseCase,
};
use application::usecases::project::create::CreateProjectUseCase;
use application::usecases::project::create_from_youtube::CreateProjectFromYoutubeUseCase;
use application::usecases::project::delete::DeleteProjectUseCase;
use application::usecases::project::get::GetProjectUseCase;
use application::usecases::project::list::ListProjectsUseCase;
use application::usecases::project::open_folder::OpenProjectFolderUseCase;
use application::usecases::project::rename::RenameProjectUseCase;
use application::usecases::transcript::get::GetTranscriptUseCase;
use application::usecases::transcript::list_youtube_tracks::ListYoutubeSubtitleTracksUseCase;
use ports::error::PortError;
use ports::job_scheduler::JobSchedulerPort;
use ports::source::{DownloadSubtitleRequest, SubtitleSourcePort};

#[derive(Clone)]
pub enum RuntimeSubtitleSource {
    Production(YtDlpAdapter),
    #[cfg(feature = "native-e2e")]
    NativeE2e(crate::bootstrap::native_e2e_subtitle_source::NativeE2ePausedSubtitleSource),
}

#[async_trait::async_trait]
impl SubtitleSourcePort for RuntimeSubtitleSource {
    async fn list_subtitles(
        &self,
        source: &domain::media::MediaSource,
    ) -> Result<Vec<domain::media::SubtitleTrack>, ports::error::PortError> {
        match self {
            Self::Production(adapter) => adapter.list_subtitles(source).await,
            #[cfg(feature = "native-e2e")]
            Self::NativeE2e(adapter) => adapter.list_subtitles(source).await,
        }
    }

    async fn download_subtitle(
        &self,
        request: DownloadSubtitleRequest,
    ) -> Result<domain::media::Artifact, ports::error::PortError> {
        match self {
            Self::Production(adapter) => adapter.download_subtitle(request).await,
            #[cfg(feature = "native-e2e")]
            Self::NativeE2e(adapter) => adapter.download_subtitle(request).await,
        }
    }
}

pub struct AppUseCases {
    pub ui_preferences: application::usecases::system::ui_preferences::UiPreferencesUseCase,
    pub artifact_recovery: application::usecases::artifact::recovery::ArtifactRecoveryUseCase,
    pub project_avatar: application::usecases::project::avatar::ProjectAvatarUseCase,
    pub list_project_artifacts: ListProjectArtifactsUseCase<RuntimeArtifactIndex>,
    pub resolve_artifact_path:
        ResolveArtifactPathUseCase<RuntimeArtifactIndex, RuntimeArtifactStore>,
    pub probe_local_media: ProbeLocalMediaUseCase<FfprobeAdapter>,
    pub import_local_media:
        ImportLocalMediaUseCase<RuntimeProjectRepository, FfprobeAdapter, RuntimeArtifactStore>,
    pub create_project: CreateProjectUseCase<RuntimeProjectRepository>,
    pub create_project_from_youtube: CreateProjectFromYoutubeUseCase<
        RuntimeProjectRepository,
        YtDlpAdapter,
        RuntimeArtifactStore,
    >,
    pub get_project: GetProjectUseCase<RuntimeProjectRepository>,
    pub list_projects: ListProjectsUseCase<RuntimeProjectRepository>,
    pub open_project_folder:
        OpenProjectFolderUseCase<RuntimeProjectRepository, adapters_tauri::ProjectWorkspaceOpener>,
    pub rename_project: RenameProjectUseCase<RuntimeProjectRepository>,
    pub delete_project: DeleteProjectUseCase,
    pub start_mock_pipeline: StartMockPipelineUseCase<
        RuntimeProjectRepository,
        RuntimeSubtitleSource,
        RuntimeArtifactStore,
    >,
    pub get_transcript: GetTranscriptUseCase<RuntimeProjectRepository>,
    pub list_youtube_subtitle_tracks:
        ListYoutubeSubtitleTracksUseCase<RuntimeProjectRepository, YtDlpAdapter>,
    pub list_jobs: ListJobsUseCase,
    pub cancel_job: CancelJobUseCase,
}

pub(super) struct AppUseCaseDependencies {
    pub(super) ui_preferences: Arc<dyn ports::ui_preferences::UiPreferencesRepository>,
    pub(super) artifact_recovery: Arc<dyn ports::artifact_recovery::ArtifactRecoveryRepository>,
    pub(super) projects_root: std::path::PathBuf,
    pub(super) project_repo: RuntimeProjectRepository,
    pub(super) project_avatar_repo: Arc<dyn ports::project_avatar::ProjectAvatarRepository>,
    pub(super) artifact_index: RuntimeArtifactIndex,
    pub(super) artifact_store: RuntimeArtifactStore,
    pub(super) storage_uow: RuntimeStorageUnitOfWork,
    pub(super) job_scheduler: Arc<dyn JobSchedulerPort>,
    pub(super) workspace_port: Arc<dyn ports::workspace::TempWorkspacePort>,
    pub(super) job_runtime: Arc<dyn ports::job_runtime_control::JobRuntimeControlPort>,
    pub(super) youtube_imports: Arc<dyn ports::youtube_import::YoutubeImportJournal>,
}

pub(super) fn setup_usecases(
    app: &AppHandle,
    dependencies: AppUseCaseDependencies,
) -> Result<(), PortError> {
    let AppUseCaseDependencies {
        ui_preferences,
        artifact_recovery,
        projects_root,
        project_repo,
        project_avatar_repo,
        artifact_index,
        artifact_store,
        storage_uow,
        job_scheduler,
        workspace_port,
        job_runtime,
        youtube_imports,
    } = dependencies;
    let ytdlp_candidates = crate::bootstrap::media_tools::resolve_ytdlp_candidates(app);
    let ytdlp_adapter = YtDlpAdapter::new(ytdlp_candidates).with_ffmpeg_candidates(
        crate::bootstrap::media_tools::resolve_ffmpeg_candidates(app),
    );
    #[cfg(feature = "native-e2e")]
    let ytdlp_adapter = ytdlp_adapter.with_native_e2e_url(
        std::env::var("AURALIS_NATIVE_E2E_YTDLP_URL").map_err(|_| PortError::Unexpected {
            message: "AURALIS_NATIVE_E2E_YTDLP_URL is required for native E2E".to_string(),
        })?,
    );
    #[cfg(not(feature = "native-e2e"))]
    let pipeline_subtitle_source = RuntimeSubtitleSource::Production(ytdlp_adapter.clone());
    #[cfg(feature = "native-e2e")]
    let pipeline_subtitle_source = {
        use crate::bootstrap::native_e2e_subtitle_source::NativeE2ePausedSubtitleSource;
        NativeE2ePausedSubtitleSource::reset();
        RuntimeSubtitleSource::NativeE2e(NativeE2ePausedSubtitleSource)
    };

    let probe = FfprobeAdapter::new(crate::bootstrap::media_tools::resolve_ffprobe_candidates(
        app,
    ));

    let locks = Arc::new(application::usecases::project::lifecycle::ProjectLifecycleLocks::new());
    let project_workspace = adapters_tauri::ProjectWorkspaceOpener::new(app.clone(), projects_root);

    let usecases = AppUseCases {
        ui_preferences: application::usecases::system::ui_preferences::UiPreferencesUseCase::new(
            ui_preferences,
        ),
        artifact_recovery: application::usecases::artifact::recovery::ArtifactRecoveryUseCase::new(
            artifact_recovery,
        ),
        project_avatar: application::usecases::project::avatar::ProjectAvatarUseCase::new(
            project_avatar_repo,
        ),
        list_project_artifacts: ListProjectArtifactsUseCase::new(artifact_index.clone()),
        resolve_artifact_path: ResolveArtifactPathUseCase::new(
            artifact_index.clone(),
            artifact_store.clone(),
        ),
        probe_local_media: ProbeLocalMediaUseCase::new(probe.clone()),
        import_local_media: ImportLocalMediaUseCase::new(
            project_repo.clone(),
            probe.clone(),
            storage_uow.clone(),
            artifact_store.clone(),
            locks.clone(),
        ),
        create_project: CreateProjectUseCase::new(project_repo.clone()),
        create_project_from_youtube: CreateProjectFromYoutubeUseCase::new(
            project_repo.clone(),
            ytdlp_adapter.clone(),
            artifact_store.clone(),
            youtube_imports,
            workspace_port.clone(),
            locks.clone(),
        ),
        get_project: GetProjectUseCase::new(project_repo.clone()),
        list_projects: ListProjectsUseCase::new(project_repo.clone()),
        open_project_folder: OpenProjectFolderUseCase::new(
            project_repo.clone(),
            project_workspace,
            locks.clone(),
        ),
        rename_project: RenameProjectUseCase::new(project_repo.clone()),
        delete_project: DeleteProjectUseCase::new(
            storage_uow.clone(),
            job_runtime.clone(),
            locks.clone(),
        ),
        start_mock_pipeline: StartMockPipelineUseCase::new(StartMockPipelineDependencies {
            project_repo: project_repo.clone(),
            job_scheduler: job_scheduler.clone(),
            storage_uow: storage_uow.clone(),
            subtitle_source: pipeline_subtitle_source,
            artifact_store: artifact_store.clone(),
            workspace_port: workspace_port.clone(),
            locks: locks.clone(),
            job_runtime: job_runtime.clone(),
        }),
        get_transcript: GetTranscriptUseCase::new(project_repo.clone()),
        list_youtube_subtitle_tracks: ListYoutubeSubtitleTracksUseCase::new(
            project_repo.clone(),
            ytdlp_adapter.clone(),
        ),
        list_jobs: ListJobsUseCase::new(job_scheduler.clone()),
        cancel_job: CancelJobUseCase::new(job_scheduler.clone()),
    };

    app.manage(Arc::new(usecases));
    Ok(())
}
