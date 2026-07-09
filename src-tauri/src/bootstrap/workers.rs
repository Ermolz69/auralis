use crate::state::{RuntimeArtifactIndex, RuntimeArtifactStore};
use adapters_storage::sqlite::SqliteOutboxRepository;
use application::worker::outbox::OutboxWorker;
use std::sync::Arc;

pub fn spawn_outbox_worker(
    outbox_repo: SqliteOutboxRepository,
    artifact_store: RuntimeArtifactStore,
    artifact_index: RuntimeArtifactIndex,
) {
    let worker = OutboxWorker::new(outbox_repo, artifact_store, artifact_index);
    let (_shutdown_tx, shutdown_rx) = tokio::sync::mpsc::channel(1);
    tauri::async_runtime::spawn(Arc::new(worker).run_loop(shutdown_rx));
}
