use crate::state::{RuntimeArtifactIndex, RuntimeArtifactStore};
use adapters_storage::sqlite::SqliteOutboxRepository;
use application::worker::outbox::OutboxWorker;
use std::sync::Arc;

pub struct OutboxWorkerShutdown(pub tokio::sync::mpsc::Sender<()>);

pub fn spawn_outbox_worker(
    outbox_repo: SqliteOutboxRepository,
    artifact_store: RuntimeArtifactStore,
    artifact_index: RuntimeArtifactIndex,
    uow: Arc<dyn ports::transaction::StorageUnitOfWork>,
    event_publisher: Arc<dyn ports::events::AppEventPublisher>,
    workspace_port: Arc<dyn ports::workspace::TempWorkspacePort>,
) -> OutboxWorkerShutdown {
    let worker = OutboxWorker::new(
        outbox_repo,
        artifact_store,
        artifact_index,
        uow,
        event_publisher,
        workspace_port,
    );
    let (shutdown_tx, shutdown_rx) = tokio::sync::mpsc::channel(1);
    tauri::async_runtime::spawn(Arc::new(worker).run_loop(shutdown_rx));
    OutboxWorkerShutdown(shutdown_tx)
}
