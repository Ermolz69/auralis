use crate::state::{
    RuntimeArtifactIndex, RuntimeArtifactStore, RuntimeProjectRepository, RuntimeStorageUnitOfWork,
};
use jobs::manager::JobManager;
use ports::error::PortError;
use ports::job_scheduler::JobLifecycleEvent;
use ports::repository::JobRepository;
use std::sync::Arc;

pub struct RuntimeServices {
    pub(crate) library: Arc<dyn ports::library::LibraryRepository>,
    pub(crate) ui_preferences: Arc<dyn ports::ui_preferences::UiPreferencesRepository>,
    pub(crate) _storage_lease: Arc<super::app_root_lease::AppRootLease>,
    pub youtube_imports: Arc<dyn ports::youtube_import::YoutubeImportJournal>,
    pub project_repo: RuntimeProjectRepository,
    pub project_avatar_repo: Arc<dyn ports::project_avatar::ProjectAvatarRepository>,
    pub job_repo: Arc<dyn JobRepository>,
    pub job_executions: Arc<dyn ports::job_execution::JobExecutionStore>,
    pub job_query: Arc<dyn ports::job_query::JobQueryPort>,
    pub artifact_index: RuntimeArtifactIndex,
    pub artifact_finalization: Arc<dyn ports::artifact_finalization::ArtifactFinalizationLookup>,
    pub artifact_store: RuntimeArtifactStore,
    pub storage_uow: RuntimeStorageUnitOfWork,
    pub translation_links: Arc<dyn ports::translation_links::TranslationLinkStore>,
    pub translation_edit_intents:
        Arc<dyn ports::translation_edit_intents::TranslationEditIntentStore>,
    pub translation_reviews: Arc<dyn ports::translation_reviews::TranslationReviewStore>,
    pub translation_host_jobs: Arc<dyn ports::translation_host_job::TranslationHostJobStore>,
    pub speech: Arc<dyn ports::speech_workflow::SpeechWorkflowStore>,
}

pub fn build_job_scheduler(
    job_repo: Arc<dyn JobRepository>,
    storage_uow: Arc<dyn ports::transaction::StorageUnitOfWork>,
    emitter: Arc<dyn Fn(JobLifecycleEvent) + Send + Sync>,
    job_executions: Arc<dyn ports::job_execution::JobExecutionStore>,
) -> Result<Arc<JobManager>, PortError> {
    let manager_impl = JobManager::new(job_repo, storage_uow, Some(emitter))
        .with_queue(job_executions, jobs::queue::JobQueueConfig::default())?;
    tauri::async_runtime::block_on(manager_impl.load_recent_jobs(100))?;
    Ok(Arc::new(manager_impl))
}
