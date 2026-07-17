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
pub struct ManagedJobEventBridge(
    pub std::sync::Mutex<Option<adapters_tauri::job_event_bridge::JobEventBridgeHandle>>,
);
pub struct ManagedTracingGuard(
    pub std::sync::Mutex<Option<crate::observability::init::TracingGuard>>,
);
