use crate::state::{
    RuntimeArtifactIndex, RuntimeArtifactStore, RuntimeProjectRepository, RuntimeStorageUnitOfWork,
};
use jobs::manager::JobManager;
use ports::job_scheduler::{JobLifecycleEvent, JobSchedulerPort};
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
    emitter: Arc<dyn Fn(JobLifecycleEvent) + Send + Sync>,
) -> Arc<dyn JobSchedulerPort> {
    let manager_impl = JobManager::new(job_repo, Some(emitter));
    tauri::async_runtime::block_on(manager_impl.load_recent_jobs(100)).ok();
    Arc::new(manager_impl)
}
