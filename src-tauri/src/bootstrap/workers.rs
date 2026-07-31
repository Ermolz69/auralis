use crate::state::{RuntimeArtifactIndex, RuntimeArtifactStore};
use adapters_storage::sqlite::SqliteOutboxRepository;
use application::worker::outbox::OutboxWorker;
use std::sync::Arc;

use application::worker::outbox::maintenance::OutboxMaintenanceConfig;

pub struct OutboxWorkerHandle {
    pub worker_task: Option<tokio::task::JoinHandle<()>>,
    pub shutdown_tx: Option<tokio::sync::mpsc::Sender<()>>,
}

impl OutboxWorkerHandle {
    pub fn into_shutdown_parts(
        mut self,
    ) -> (
        Option<tokio::sync::mpsc::Sender<()>>,
        Option<tokio::task::JoinHandle<()>>,
    ) {
        (self.shutdown_tx.take(), self.worker_task.take())
    }
}

pub fn spawn_outbox_worker(
    outbox_repo: SqliteOutboxRepository,
    artifact_store: RuntimeArtifactStore,
    artifact_index: RuntimeArtifactIndex,
    uow: Arc<dyn ports::transaction::StorageUnitOfWork>,
    event_publisher: Arc<dyn ports::events::AppEventPublisher>,
    workspace_port: Arc<dyn ports::workspace::TempWorkspacePort>,
    config: OutboxMaintenanceConfig,
) -> OutboxWorkerHandle {
    let worker = OutboxWorker::new(
        outbox_repo,
        artifact_store,
        artifact_index,
        uow,
        event_publisher,
        workspace_port,
        config,
    );
    let (shutdown_tx, shutdown_rx) = tokio::sync::mpsc::channel(1);
    let worker_task = tokio::spawn(Arc::new(worker).run_loop(shutdown_rx));
    OutboxWorkerHandle {
        worker_task: Some(worker_task),
        shutdown_tx: Some(shutdown_tx),
    }
}
