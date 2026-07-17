#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{Notify, watch};

use crate::worker::outbox::OutboxWorker;
use crate::worker::outbox::maintenance::{MaintenanceCoordinator, MaintenanceStepOutcome};
use domain::outbox::{OutboxMessage, OutboxMessageId, OutboxPayload, WorkspaceKey};
use domain::project::ProjectId;
use ports::error::PortError;
use ports::repository::OutboxRepository;

// Mock dependencies
#[derive(Clone)]
struct MockOutboxRepository {
    pub mark_done_calls: Arc<Mutex<usize>>,
    pub mark_failed_calls: Arc<Mutex<usize>>,
    pub messages_to_return: Arc<Mutex<Vec<OutboxMessage>>>,
    pub mark_processing_result: Arc<Mutex<Result<bool, PortError>>>,
    pub mark_done_result: Arc<Mutex<Result<(), PortError>>>,
    pub mark_failed_result: Arc<Mutex<Result<(), PortError>>>,
    pub prune_calls: Arc<Mutex<usize>>,
    pub prune_result: Arc<Mutex<Result<ports::repository::OutboxPruneReport, PortError>>>,
}

impl Default for MockOutboxRepository {
    fn default() -> Self {
        Self {
            mark_done_calls: Arc::new(Mutex::new(0)),
            mark_failed_calls: Arc::new(Mutex::new(0)),
            messages_to_return: Arc::new(Mutex::new(Vec::new())),
            mark_processing_result: Arc::new(Mutex::new(Ok(true))),
            mark_done_result: Arc::new(Mutex::new(Ok(()))),
            mark_failed_result: Arc::new(Mutex::new(Ok(()))),
            prune_calls: Arc::new(Mutex::new(0)),
            prune_result: Arc::new(Mutex::new(Ok(ports::repository::OutboxPruneReport {
                done_deleted: 0,
                dead_deleted: 0,
            }))),
        }
    }
}

use ports::repository::FetchPendingResult;

#[async_trait]
impl OutboxRepository for MockOutboxRepository {
    async fn fetch_pending(&self, limit: usize) -> Result<FetchPendingResult, PortError> {
        let mut messages = self.messages_to_return.lock().unwrap();
        let count = std::cmp::min(limit, messages.len());
        let result: Vec<_> = messages.drain(0..count).collect();
        Ok(FetchPendingResult {
            messages: result,
            corrupted_isolated: 0,
            isolation_errors: 0,
        })
    }

    async fn mark_processing(
        &self,
        _id: &OutboxMessageId,
        _locked_by: &str,
    ) -> Result<bool, PortError> {
        let result = self.mark_processing_result.lock().unwrap();
        match &*result {
            Ok(b) => Ok(*b),
            Err(e) => Err(PortError::Io {
                message: e.to_string(),
            }),
        }
    }

    async fn mark_done(&self, _id: &OutboxMessageId) -> Result<(), PortError> {
        *self.mark_done_calls.lock().unwrap() += 1;
        let result = self.mark_done_result.lock().unwrap();
        match &*result {
            Ok(_) => Ok(()),
            Err(e) => Err(PortError::Io {
                message: e.to_string(),
            }),
        }
    }

    async fn mark_failed(&self, _id: &OutboxMessageId, _error: &str) -> Result<(), PortError> {
        *self.mark_failed_calls.lock().unwrap() += 1;
        let result = self.mark_failed_result.lock().unwrap();
        match &*result {
            Ok(_) => Ok(()),
            Err(e) => Err(PortError::Io {
                message: e.to_string(),
            }),
        }
    }

    async fn mark_dead_raw(&self, _id_raw: &str, _reason: &str) -> Result<(), PortError> {
        Ok(())
    }

    async fn prune_terminal_rows(
        &self,
        _done_before: chrono::DateTime<chrono::Utc>,
        _dead_before: chrono::DateTime<chrono::Utc>,
        _batch_limit: u32,
    ) -> Result<ports::repository::OutboxPruneReport, PortError> {
        *self.prune_calls.lock().unwrap() += 1;
        let res = self.prune_result.lock().unwrap();
        match &*res {
            Ok(r) => Ok(ports::repository::OutboxPruneReport {
                done_deleted: r.done_deleted,
                dead_deleted: r.dead_deleted,
            }),
            Err(e) => Err(PortError::Io {
                message: e.to_string(),
            }),
        }
    }
}

#[derive(Clone)]
struct MockStore {
    cleanup_calls: Arc<Mutex<usize>>,
    cleanup_res: Arc<Mutex<Result<(), PortError>>>,
    // Optional Notify gate to pause maintenance in overlap tests
    gate: Option<Arc<Notify>>,
}

impl Default for MockStore {
    fn default() -> Self {
        Self {
            cleanup_calls: Arc::new(Mutex::new(0)),
            cleanup_res: Arc::new(Mutex::new(Ok(()))),
            gate: None,
        }
    }
}

#[async_trait]
impl ports::storage::ArtifactStore for MockStore {
    async fn stage_owned_temp_file(
        &self,
        _project_id: &ProjectId,
        _kind: domain::media::ArtifactKind,
        _source_path: &std::path::Path,
        _filename_hint: Option<&str>,
    ) -> Result<ports::storage::StagedArtifact, ports::error::PortError> {
        unimplemented!()
    }
    async fn import_external_file(
        &self,
        _project_id: &ProjectId,
        _kind: domain::media::ArtifactKind,
        _source_path: &std::path::Path,
        _filename_hint: Option<&str>,
    ) -> Result<ports::storage::StagedArtifact, ports::error::PortError> {
        unimplemented!()
    }
    async fn finalize_staged_artifact(
        &self,
        _staging_key: &str,
        _final_key: &str,
    ) -> Result<(), ports::error::PortError> {
        Ok(())
    }
    async fn resolve_artifact(
        &self,
        _artifact: &domain::media::Artifact,
    ) -> Result<std::path::PathBuf, ports::error::PortError> {
        unimplemented!()
    }
    async fn delete_storage_key(&self, _storage_key: &str) -> Result<(), ports::error::PortError> {
        Ok(())
    }
    async fn delete_artifact(
        &self,
        _artifact: &domain::media::Artifact,
    ) -> Result<(), ports::error::PortError> {
        Ok(())
    }
    async fn delete_project_dir(
        &self,
        _project_id: &ProjectId,
    ) -> Result<(), ports::error::PortError> {
        Ok(())
    }
    async fn cleanup_stale_staging(
        &self,
        _max_age: std::time::Duration,
    ) -> Result<(), ports::error::PortError> {
        *self.cleanup_calls.lock().unwrap() += 1;
        if let Some(ref g) = self.gate {
            g.notified().await;
        }
        match &*self.cleanup_res.lock().unwrap() {
            Ok(_) => Ok(()),
            Err(e) => Err(PortError::Io {
                message: e.to_string(),
            }),
        }
    }
}

#[derive(Clone, Default)]
struct MockIndex;
#[async_trait]
impl ports::artifact_index::ArtifactIndex for MockIndex {
    async fn add(
        &self,
        _project_id: &ProjectId,
        _artifact: &domain::media::Artifact,
    ) -> Result<(), ports::error::PortError> {
        unimplemented!()
    }
    async fn get(
        &self,
        _id: &domain::media::ArtifactId,
    ) -> Result<Option<domain::media::Artifact>, ports::error::PortError> {
        unimplemented!()
    }
    async fn check_exists(
        &self,
        _id: &domain::media::ArtifactId,
    ) -> Result<bool, ports::error::PortError> {
        unimplemented!()
    }
    async fn list_by_project(
        &self,
        _project_id: &ProjectId,
    ) -> Result<Vec<domain::media::Artifact>, ports::error::PortError> {
        unimplemented!()
    }
    async fn list_by_project_and_kind(
        &self,
        _project_id: &ProjectId,
        _kind: domain::media::ArtifactKind,
    ) -> Result<Vec<domain::media::Artifact>, ports::error::PortError> {
        unimplemented!()
    }
    async fn delete(&self, _id: &domain::media::ArtifactId) -> Result<(), ports::error::PortError> {
        Ok(())
    }
    async fn update_state(
        &self,
        _id: &domain::media::ArtifactId,
        _state: domain::media::ArtifactState,
        _ready_at: Option<domain::chrono::DateTime<domain::chrono::Utc>>,
    ) -> Result<(), ports::error::PortError> {
        Ok(())
    }
}

#[derive(Clone, Default)]
struct MockUow;
#[async_trait]
impl ports::transaction::StorageUnitOfWork for MockUow {
    async fn commit_artifact_finalize(
        &self,
        _command: ports::transaction::CommitArtifactFinalize,
    ) -> Result<ports::transaction::CommitArtifactFinalizeResult, ports::error::PortError> {
        unimplemented!()
    }
    async fn commit_transcript_import(
        &self,
        _command: ports::transaction::CommitTranscriptImport,
    ) -> Result<(), ports::error::PortError> {
        unimplemented!()
    }
    async fn commit_staged_artifact_write(
        &self,
        _command: ports::transaction::CommitStagedArtifactWrite,
    ) -> Result<(), ports::error::PortError> {
        unimplemented!()
    }
    async fn commit_managed_source_import(
        &self,
        _command: ports::transaction::CommitManagedSourceImport,
    ) -> Result<(), ports::error::PortError> {
        unimplemented!()
    }
    async fn commit_project_delete(
        &self,
        _command: ports::transaction::CommitProjectDelete,
    ) -> Result<ports::transaction::CommitProjectDeleteResult, ports::error::PortError> {
        unimplemented!()
    }
    async fn commit_job_update(
        &self,
        _command: ports::transaction::CommitJobUpdate,
    ) -> Result<(), ports::error::PortError> {
        unimplemented!()
    }
    async fn commit_pipeline_start(
        &self,
        _command: ports::transaction::CommitPipelineStart,
    ) -> Result<(), ports::error::PortError> {
        unimplemented!()
    }
    async fn commit_pipeline_start_failure(
        &self,
        _command: ports::transaction::CommitPipelineStartFailure,
    ) -> Result<(), ports::error::PortError> {
        unimplemented!()
    }
    async fn commit_terminal_job_update(
        &self,
        _command: ports::transaction::CommitTerminalJobUpdate,
    ) -> Result<(), ports::error::PortError> {
        unimplemented!()
    }
    async fn apply_terminal_lifecycle_conditionally(
        &self,
        _command: ports::transaction::ApplyTerminalLifecycle,
    ) -> Result<domain::project::status::TerminalTransitionResult, ports::error::PortError> {
        Ok(domain::project::status::TerminalTransitionResult::IgnoredStale)
    }
}

#[derive(Default)]
struct MockEventPublisher;
#[async_trait]
impl ports::events::AppEventPublisher for MockEventPublisher {
    async fn publish_project_updated(
        &self,
        _project_id: &str,
    ) -> Result<(), ports::error::PortError> {
        Ok(())
    }
    async fn publish_transcript_ready(
        &self,
        _project_id: &str,
        _job_id: &str,
    ) -> Result<(), ports::error::PortError> {
        Ok(())
    }
}

#[derive(Clone)]
struct MockWorkspacePort {
    cleanup_calls: Arc<Mutex<usize>>,
    cleanup_res: Arc<Mutex<Result<ports::workspace::WorkspaceCleanupReport, PortError>>>,
}

impl Default for MockWorkspacePort {
    fn default() -> Self {
        Self {
            cleanup_calls: Arc::new(Mutex::new(0)),
            cleanup_res: Arc::new(Mutex::new(Ok(ports::workspace::WorkspaceCleanupReport {
                deleted_count: 0,
                failed_count: 0,
            }))),
        }
    }
}

#[async_trait]
impl ports::workspace::TempWorkspacePort for MockWorkspacePort {
    async fn create_allocation(
        &self,
        _project_id: &ProjectId,
        _purpose: &str,
    ) -> Result<ports::workspace::WorkspaceAllocation, PortError> {
        unimplemented!()
    }

    async fn delete_allocation(&self, _key: &WorkspaceKey) -> Result<(), PortError> {
        Ok(())
    }

    async fn resolve_key(&self, _key: &WorkspaceKey) -> Result<std::path::PathBuf, PortError> {
        unimplemented!()
    }

    async fn cleanup_stale_allocations(
        &self,
        _age_threshold: std::time::Duration,
    ) -> Result<ports::workspace::WorkspaceCleanupReport, PortError> {
        *self.cleanup_calls.lock().unwrap() += 1;
        match &*self.cleanup_res.lock().unwrap() {
            Ok(report) => Ok(ports::workspace::WorkspaceCleanupReport {
                deleted_count: report.deleted_count,
                failed_count: report.failed_count,
            }),
            Err(e) => Err(PortError::Io {
                message: e.to_string(),
            }),
        }
    }
}

fn create_worker(
    repo: MockOutboxRepository,
) -> OutboxWorker<MockOutboxRepository, MockStore, MockIndex, MockUow> {
    OutboxWorker::new(
        repo,
        MockStore::default(),
        MockIndex,
        MockUow,
        Arc::new(MockEventPublisher),
        Arc::new(MockWorkspacePort::default()),
        super::maintenance::OutboxMaintenanceConfig::try_default().unwrap(),
    )
}

#[tokio::test]
async fn worker_processes_success() {
    let repo = MockOutboxRepository::default();
    repo.messages_to_return
        .lock()
        .unwrap()
        .push(OutboxMessage::new(OutboxPayload::DeleteWorkspaceFile {
            workspace_key: WorkspaceKey::new("tmp/test_id/purpose_id").unwrap(),
        }));

    *repo.mark_processing_result.lock().unwrap() = Ok(true);
    *repo.mark_done_result.lock().unwrap() = Ok(());

    let worker = create_worker(repo.clone());
    let report = worker.process_pending_messages(10).await.unwrap();

    assert_eq!(report.fetched, 1);
    assert_eq!(report.claimed, 1);
    assert_eq!(report.completed, 1);
    assert_eq!(report.storage_errors, 0);

    assert_eq!(*repo.mark_done_calls.lock().unwrap(), 1);
    assert_eq!(*repo.mark_failed_calls.lock().unwrap(), 0);
}

#[tokio::test]
async fn worker_handles_claim_conflict() {
    let repo = MockOutboxRepository::default();
    repo.messages_to_return
        .lock()
        .unwrap()
        .push(OutboxMessage::new(OutboxPayload::DeleteWorkspaceFile {
            workspace_key: WorkspaceKey::new("tmp/test_id/purpose_id").unwrap(),
        }));

    *repo.mark_processing_result.lock().unwrap() = Ok(false);

    let worker = create_worker(repo.clone());
    let report = worker.process_pending_messages(10).await.unwrap();

    assert_eq!(report.fetched, 1);
    assert_eq!(report.claim_conflicts, 1);
    assert_eq!(report.claimed, 0);
    assert_eq!(report.completed, 0);
    assert_eq!(report.storage_errors, 0);

    assert_eq!(*repo.mark_done_calls.lock().unwrap(), 0);
    assert_eq!(*repo.mark_failed_calls.lock().unwrap(), 0);
}

#[tokio::test]
async fn worker_handles_claim_error() {
    let repo = MockOutboxRepository::default();
    repo.messages_to_return
        .lock()
        .unwrap()
        .push(OutboxMessage::new(OutboxPayload::DeleteWorkspaceFile {
            workspace_key: WorkspaceKey::new("tmp/test_id/purpose_id").unwrap(),
        }));

    *repo.mark_processing_result.lock().unwrap() = Err(PortError::Io {
        message: "db down".to_string(),
    });

    let worker = create_worker(repo.clone());
    let report = worker.process_pending_messages(10).await.unwrap();

    assert_eq!(report.fetched, 1);
    assert_eq!(report.claimed, 0);
    assert_eq!(report.storage_errors, 1);

    assert_eq!(*repo.mark_done_calls.lock().unwrap(), 0);
    assert_eq!(*repo.mark_failed_calls.lock().unwrap(), 0);
}

#[tokio::test]
async fn worker_handles_mark_done_error() {
    let repo = MockOutboxRepository::default();
    repo.messages_to_return
        .lock()
        .unwrap()
        .push(OutboxMessage::new(OutboxPayload::DeleteWorkspaceFile {
            workspace_key: WorkspaceKey::new("tmp/test_id/purpose_id").unwrap(),
        }));

    *repo.mark_processing_result.lock().unwrap() = Ok(true);
    *repo.mark_done_result.lock().unwrap() = Err(PortError::Io {
        message: "db down".to_string(),
    });

    let worker = create_worker(repo.clone());
    let report = worker.process_pending_messages(10).await.unwrap();

    assert_eq!(report.fetched, 1);
    assert_eq!(report.claimed, 1);
    assert_eq!(report.completed, 0);
    assert_eq!(report.storage_errors, 1);

    assert_eq!(*repo.mark_done_calls.lock().unwrap(), 1);
    assert_eq!(*repo.mark_failed_calls.lock().unwrap(), 0);
}

// ==========================================
// MaintenanceCoordinator Tests
// ==========================================

#[tokio::test]
async fn test_coordinator_step_failures() {
    let repo = MockOutboxRepository::default();
    let store = MockStore::default();
    let workspace = MockWorkspacePort::default();

    // Force staging janitor error
    *store.cleanup_res.lock().unwrap() = Err(PortError::Io {
        message: "staging fail".to_string(),
    });

    // Force pruning error
    *repo.prune_result.lock().unwrap() = Err(PortError::Io {
        message: "prune fail".to_string(),
    });

    let config = super::maintenance::OutboxMaintenanceConfig::try_default().unwrap();
    let coordinator = MaintenanceCoordinator::new(repo, store, Arc::new(workspace), config);

    let (_, cancel_rx) = watch::channel(false);
    let report = coordinator.run_maintenance(cancel_rx).await;

    assert_eq!(report.staging_cleanup, MaintenanceStepOutcome::Failed);
    assert_eq!(report.workspace_cleanup, MaintenanceStepOutcome::Succeeded);
    assert_eq!(report.pruning, MaintenanceStepOutcome::Failed);
    assert!(!report.cancelled);
}

#[tokio::test]
async fn test_coordinator_max_batches() {
    let repo = MockOutboxRepository::default();
    let store = MockStore::default();
    let workspace = MockWorkspacePort::default();

    // Returning a full batch to keep pruning loop going
    *repo.prune_result.lock().unwrap() = Ok(ports::repository::OutboxPruneReport {
        done_deleted: 500, // equal to per_status_batch_limit
        dead_deleted: 500,
    });

    let config = super::maintenance::OutboxMaintenanceConfig::try_default().unwrap();
    let coordinator = MaintenanceCoordinator::new(repo.clone(), store, Arc::new(workspace), config);

    let (_, cancel_rx) = watch::channel(false);
    let report = coordinator.run_maintenance(cancel_rx).await;

    assert_eq!(report.batches_run, 10); // max_batches = 10
    assert_eq!(*repo.prune_calls.lock().unwrap(), 10);
    assert_eq!(report.pruning, MaintenanceStepOutcome::Succeeded);
}

#[tokio::test]
async fn test_coordinator_cancellation() {
    let repo = MockOutboxRepository::default();
    let store = MockStore::default();
    let workspace = MockWorkspacePort::default();

    *repo.prune_result.lock().unwrap() = Ok(ports::repository::OutboxPruneReport {
        done_deleted: 500,
        dead_deleted: 500,
    });

    let config = super::maintenance::OutboxMaintenanceConfig::try_default().unwrap();
    let coordinator = MaintenanceCoordinator::new(repo, store, Arc::new(workspace), config);

    let (cancel_tx, cancel_rx) = watch::channel(false);

    // Cancel after 2 batches by checking calls or spawning and cancelling asynchronously
    let handle = tokio::spawn(async move { coordinator.run_maintenance(cancel_rx).await });

    // Let the loop start
    tokio::task::yield_now().await;
    let _ = cancel_tx.send(true);

    let report = handle.await.unwrap();
    assert!(report.cancelled);
    assert!(report.batches_run < 10);
}

#[tokio::test]
async fn test_coordinator_separate_cutoffs() {
    let repo = MockOutboxRepository::default();
    let store = MockStore::default();
    let workspace = MockWorkspacePort::default();

    let config = super::maintenance::OutboxMaintenanceConfig::try_default().unwrap();
    let coordinator = MaintenanceCoordinator::new(repo.clone(), store, Arc::new(workspace), config);

    let (_, cancel_rx) = watch::channel(false);
    let now = chrono::DateTime::parse_from_rfc3339("2026-07-17T20:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    let _report = coordinator.run_maintenance_at(now, cancel_rx).await;

    // Verify pruning calls were made
    assert_eq!(*repo.prune_calls.lock().unwrap(), 1);
}

// ==========================================
// OutboxWorker::run_loop Tests
// ==========================================

#[tokio::test(start_paused = true)]
async fn test_run_loop_startup_maintenance() {
    let repo = MockOutboxRepository::default();
    let store = MockStore::default();
    let workspace = MockWorkspacePort::default();

    let mut config = super::maintenance::OutboxMaintenanceConfig::try_default().unwrap();
    config.run_on_startup = true;
    config.interval = Duration::from_secs(10);

    let worker = Arc::new(OutboxWorker::new(
        repo.clone(),
        store.clone(),
        MockIndex,
        MockUow,
        Arc::new(MockEventPublisher),
        Arc::new(workspace.clone()),
        config,
    ));

    let (shutdown_tx, shutdown_rx) = tokio::sync::mpsc::channel(1);
    let worker_clone = worker.clone();
    let handle = tokio::spawn(async move {
        worker_clone.run_loop(shutdown_rx).await;
    });

    // Advance time slightly to let startup tasks run
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Startup maintenance must be executed exactly once
    assert_eq!(*store.cleanup_calls.lock().unwrap(), 1);
    assert_eq!(*workspace.cleanup_calls.lock().unwrap(), 1);

    // Shutdown worker gracefully
    let _ = shutdown_tx.send(()).await;
    let _ = handle.await;
}

#[tokio::test(start_paused = true)]
async fn test_run_loop_interval_ticks() {
    let repo = MockOutboxRepository::default();
    let store = MockStore::default();
    let workspace = MockWorkspacePort::default();

    let mut config = super::maintenance::OutboxMaintenanceConfig::try_default().unwrap();
    config.run_on_startup = false;
    config.interval = Duration::from_secs(10);

    let worker = Arc::new(OutboxWorker::new(
        repo.clone(),
        store.clone(),
        MockIndex,
        MockUow,
        Arc::new(MockEventPublisher),
        Arc::new(workspace.clone()),
        config,
    ));

    let (shutdown_tx, shutdown_rx) = tokio::sync::mpsc::channel(1);
    let worker_clone = worker.clone();
    let handle = tokio::spawn(async move {
        worker_clone.run_loop(shutdown_rx).await;
    });

    // Let the spawned task initialize and build maintenance_interval at t = 0
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Advance 5 seconds - no maintenance because start_on_startup is false, and interval is 10s
    tokio::time::advance(Duration::from_secs(5)).await;
    assert_eq!(*store.cleanup_calls.lock().unwrap(), 0);

    // Advance to 10 seconds (first interval tick)
    tokio::time::advance(Duration::from_secs(5)).await;
    // Let tasks spawn and run
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert_eq!(*store.cleanup_calls.lock().unwrap(), 1);

    // Shutdown worker
    let _ = shutdown_tx.send(()).await;
    let _ = handle.await;
}

#[tokio::test(start_paused = true)]
async fn test_run_loop_overlap_prevention() {
    let repo = MockOutboxRepository::default();
    let gate = Arc::new(Notify::new());

    // Store is gated to pause maintenance during cleanup
    let store = MockStore {
        gate: Some(gate.clone()),
        ..Default::default()
    };
    let workspace = MockWorkspacePort::default();

    let mut config = super::maintenance::OutboxMaintenanceConfig::try_default().unwrap();
    config.run_on_startup = true;
    config.interval = Duration::from_secs(10);

    let worker = Arc::new(OutboxWorker::new(
        repo.clone(),
        store.clone(),
        MockIndex,
        MockUow,
        Arc::new(MockEventPublisher),
        Arc::new(workspace.clone()),
        config,
    ));

    let (shutdown_tx, shutdown_rx) = tokio::sync::mpsc::channel(1);
    let worker_clone = worker.clone();
    let handle = tokio::spawn(async move {
        worker_clone.run_loop(shutdown_rx).await;
    });

    // Yield to let startup maintenance begin and hit the Notify gate
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert_eq!(*store.cleanup_calls.lock().unwrap(), 1);

    // Advance past the maintenance interval (10 seconds)
    tokio::time::advance(Duration::from_secs(10)).await;
    tokio::time::sleep(Duration::from_millis(10)).await;

    // The second maintenance tick should NOT spawn a new task (overlap prevention)
    assert_eq!(*store.cleanup_calls.lock().unwrap(), 1);

    // Release gate to let the first maintenance run finish
    gate.notify_one();
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Shutdown worker
    let _ = shutdown_tx.send(()).await;
    let _ = handle.await;
}

#[tokio::test(start_paused = true)]
async fn test_run_loop_delivery_during_maintenance() {
    let repo = MockOutboxRepository::default();
    let gate = Arc::new(Notify::new());

    let store = MockStore {
        gate: Some(gate.clone()),
        ..Default::default()
    };
    let workspace = MockWorkspacePort::default();

    let mut config = super::maintenance::OutboxMaintenanceConfig::try_default().unwrap();
    config.run_on_startup = true;
    config.interval = Duration::from_secs(10);

    let worker = Arc::new(OutboxWorker::new(
        repo.clone(),
        store.clone(),
        MockIndex,
        MockUow,
        Arc::new(MockEventPublisher),
        Arc::new(workspace.clone()),
        config,
    ));

    let (shutdown_tx, shutdown_rx) = tokio::sync::mpsc::channel(1);
    let worker_clone = worker.clone();
    let handle = tokio::spawn(async move {
        worker_clone.run_loop(shutdown_rx).await;
    });

    // Startup maintenance starts and blocks
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert_eq!(*store.cleanup_calls.lock().unwrap(), 1);

    // Prepare outbox messages for delivery loop (ticks every 5 seconds)
    repo.messages_to_return
        .lock()
        .unwrap()
        .push(OutboxMessage::new(OutboxPayload::DeleteWorkspaceFile {
            workspace_key: WorkspaceKey::new("tmp/test_id/purpose_id").unwrap(),
        }));

    // Advance 5 seconds to trigger delivery tick
    tokio::time::advance(Duration::from_secs(5)).await;
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Delivery must process messages even if maintenance is blocked/running
    assert_eq!(*repo.mark_done_calls.lock().unwrap(), 1);

    // Release gate
    gate.notify_one();
    tokio::time::sleep(Duration::from_millis(10)).await;

    // Shutdown worker
    let _ = shutdown_tx.send(()).await;
    let _ = handle.await;
}
