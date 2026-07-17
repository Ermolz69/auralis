#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::sync::Arc;
use std::sync::Mutex;

use async_trait::async_trait;

use crate::worker::outbox::OutboxWorker;
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
            }), // Clone error
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
        Ok(ports::repository::OutboxPruneReport {
            done_deleted: 0,
            dead_deleted: 0,
        })
    }
}

// Dummy struct for ArtifactStore, ArtifactIndex, StorageUnitOfWork
#[derive(Clone, Default)]
struct MockStore;
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
        Ok(())
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
    ) -> Result<(), ports::error::PortError> {
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

#[derive(Default)]
struct MockWorkspacePort;

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
        Ok(()) // Success for outbox tests
    }

    async fn resolve_key(&self, _key: &WorkspaceKey) -> Result<std::path::PathBuf, PortError> {
        unimplemented!()
    }

    async fn cleanup_stale_allocations(
        &self,
        _age_threshold: std::time::Duration,
    ) -> Result<ports::workspace::WorkspaceCleanupReport, PortError> {
        Ok(ports::workspace::WorkspaceCleanupReport {
            deleted_count: 0,
            failed_count: 0,
        })
    }
}

fn create_worker(
    repo: MockOutboxRepository,
) -> OutboxWorker<MockOutboxRepository, MockStore, MockIndex, MockUow> {
    OutboxWorker::new(
        repo,
        MockStore,
        MockIndex,
        MockUow,
        Arc::new(MockEventPublisher),
        Arc::new(MockWorkspacePort),
        super::maintenance::OutboxMaintenanceConfig::default_config(),
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

    // Claim fails legitimately (someone else got it)
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

    // DB error on claim
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
    // DB error on mark done
    *repo.mark_done_result.lock().unwrap() = Err(PortError::Io {
        message: "db down".to_string(),
    });

    let worker = create_worker(repo.clone());
    let report = worker.process_pending_messages(10).await.unwrap();

    assert_eq!(report.fetched, 1);
    assert_eq!(report.claimed, 1);
    assert_eq!(report.completed, 0); // Not completed successfully from our point of view
    assert_eq!(report.storage_errors, 1);

    assert_eq!(*repo.mark_done_calls.lock().unwrap(), 1);
    assert_eq!(*repo.mark_failed_calls.lock().unwrap(), 0);
}
