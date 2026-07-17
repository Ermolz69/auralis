use crate::state::{
    RuntimeArtifactIndex, RuntimeArtifactStore, RuntimeProjectRepository, RuntimeStorageUnitOfWork,
};
use jobs::manager::JobManager;
use ports::job_scheduler::JobLifecycleEvent;
use ports::repository::JobRepository;
use std::sync::Arc;

pub struct RuntimeServices {
    pub project_repo: RuntimeProjectRepository,
    pub job_repo: Arc<dyn JobRepository>,
    pub artifact_index: RuntimeArtifactIndex,
    pub artifact_store: RuntimeArtifactStore,
    pub storage_uow: RuntimeStorageUnitOfWork,
}

pub fn build_job_scheduler(
    job_repo: Arc<dyn JobRepository>,
    storage_uow: Arc<dyn ports::transaction::StorageUnitOfWork>,
    emitter: Arc<dyn Fn(JobLifecycleEvent) + Send + Sync>,
) -> Result<Arc<JobManager>, String> {
    let manager_impl = JobManager::new(job_repo, storage_uow, Some(emitter));
    tauri::async_runtime::block_on(manager_impl.load_recent_jobs(100))
        .map_err(|e| format!("Failed to load recent jobs: {}", e))?;
    Ok(Arc::new(manager_impl))
}
