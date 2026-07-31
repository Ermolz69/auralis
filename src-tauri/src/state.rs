use ports::artifact_index::ArtifactIndex;
use ports::repository::ProjectRepository;
use ports::storage::ArtifactStore;
use ports::transaction::StorageUnitOfWork;
use std::sync::Arc;

pub type RuntimeProjectRepository = Arc<dyn ProjectRepository>;
pub type RuntimeArtifactIndex = Arc<dyn ArtifactIndex>;
pub type RuntimeArtifactStore = Arc<dyn ArtifactStore>;
pub type RuntimeStorageUnitOfWork = Arc<dyn StorageUnitOfWork>;

pub struct ManagedOutboxWorker(
    pub std::sync::Mutex<Option<crate::bootstrap::workers::OutboxWorkerHandle>>,
);

impl ManagedOutboxWorker {
    pub fn take(&self) -> Option<crate::bootstrap::workers::OutboxWorkerHandle> {
        match self.0.lock() {
            Ok(mut lock) => lock.take(),
            Err(e) => {
                tracing::warn!(
                    "Poisoned mutex encountered for ManagedOutboxWorker. Attempting recovery."
                );
                e.into_inner().take()
            }
        }
    }
}

pub struct ManagedJobEventBridge(
    pub std::sync::Mutex<Option<adapters_tauri::job_event_bridge::JobEventBridgeHandle>>,
);

impl ManagedJobEventBridge {
    pub fn take(&self) -> Option<adapters_tauri::job_event_bridge::JobEventBridgeHandle> {
        match self.0.lock() {
            Ok(mut lock) => lock.take(),
            Err(e) => {
                tracing::warn!(
                    "Poisoned mutex encountered for ManagedJobEventBridge. Attempting recovery."
                );
                e.into_inner().take()
            }
        }
    }
}

pub struct ManagedTracingGuard(
    pub std::sync::Mutex<Option<crate::observability::init::TracingGuard>>,
);

impl ManagedTracingGuard {
    pub fn take(&self) -> Option<crate::observability::init::TracingGuard> {
        match self.0.lock() {
            Ok(mut lock) => lock.take(),
            Err(e) => {
                tracing::warn!(
                    "Poisoned mutex encountered for ManagedTracingGuard. Attempting recovery."
                );
                e.into_inner().take()
            }
        }
    }
}

pub struct ManagedJobRuntime(pub Arc<dyn ports::job_runtime_control::JobRuntimeControlPort>);
