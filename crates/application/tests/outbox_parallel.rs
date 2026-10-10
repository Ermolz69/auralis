use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use adapters_storage::{
    local::LocalTempWorkspace,
    sqlite::{
        SqliteArtifactIndex, SqliteOutboxRepository, SqliteStorageUnitOfWork, connect_sqlite,
    },
};
use application::worker::outbox::{OutboxWorker, maintenance::OutboxMaintenanceConfig};
use domain::{
    media::{Artifact, ArtifactKind},
    outbox::{OutboxMessage, OutboxPayload, WorkspaceKey},
    project::ProjectId,
};
use ports::{
    error::PortError,
    repository::OutboxRepository,
    storage::{ArtifactStore, StagedArtifact},
};
use tokio::sync::{Barrier, Notify};

struct SelectiveGate {
    slow_project: ProjectId,
    release_slow: Notify,
    fast_seen: Notify,
    fast_count: AtomicUsize,
}

#[derive(Clone)]
struct BarrierStore {
    gate: Arc<Barrier>,
    entered: Arc<AtomicUsize>,
    panic_on_delete: bool,
    selective: Option<Arc<SelectiveGate>>,
}

#[async_trait::async_trait]
impl ArtifactStore for BarrierStore {
    async fn stage_owned_temp_file(
        &self,
        _: &ProjectId,
        _: ArtifactKind,
        _: &Path,
        _: Option<&str>,
    ) -> Result<StagedArtifact, PortError> {
        unreachable!()
    }
    async fn import_external_file(
        &self,
        _: &ProjectId,
        _: ArtifactKind,
        _: &Path,
        _: Option<&str>,
    ) -> Result<StagedArtifact, PortError> {
        unreachable!()
    }
    async fn finalize_staged_artifact(
        &self,
        _: &str,
        _: &str,
        _: Option<u64>,
    ) -> Result<(), PortError> {
        unreachable!()
    }
    async fn resolve_artifact(&self, _: &Artifact) -> Result<PathBuf, PortError> {
        unreachable!()
    }
    async fn delete_storage_key(&self, _: &str) -> Result<(), PortError> {
        unreachable!()
    }
    async fn delete_artifact(&self, _: &Artifact) -> Result<(), PortError> {
        unreachable!()
    }
    async fn delete_project_dir(&self, project_id: &ProjectId) -> Result<(), PortError> {
        self.entered.fetch_add(1, Ordering::SeqCst);
        if let Some(selective) = &self.selective {
            if project_id == &selective.slow_project {
                selective.release_slow.notified().await;
            } else {
                selective.fast_count.fetch_add(1, Ordering::SeqCst);
                selective.fast_seen.notify_one();
            }
        } else {
            self.gate.wait().await;
        }
        assert!(!self.panic_on_delete, "simulated handler panic");
        Ok(())
    }
    async fn cleanup_stale_staging(&self, _: std::time::Duration) -> Result<(), PortError> {
        Ok(())
    }
    async fn stage_owned_workspace_file(
        &self,
        _: &ProjectId,
        _: ArtifactKind,
        _: &dyn ports::workspace::TempWorkspacePort,
        _: &WorkspaceKey,
        _: &str,
        _: Option<&str>,
    ) -> Result<StagedArtifact, PortError> {
        unreachable!()
    }
}

struct NoEvents;

#[async_trait::async_trait]
impl ports::events::AppEventPublisher for NoEvents {
    async fn publish_project_updated(&self, _: &str) -> Result<(), PortError> {
        Ok(())
    }
    async fn publish_transcript_ready(&self, _: &str, _: &str) -> Result<(), PortError> {
        Ok(())
    }
}

async fn insert(pool: &sqlx::SqlitePool, project: ProjectId) -> Result<(), Box<dyn Error>> {
    let message = OutboxMessage::new(OutboxPayload::DeleteProjectArtifactDir {
        project_id: project.clone(),
    });
    let sequence: i64 = sqlx::query_scalar(
        "UPDATE outbox_sequence SET value = value + 1 WHERE singleton = 1 RETURNING value",
    )
    .fetch_one(pool)
    .await?;
    sqlx::query(
        "INSERT INTO outbox_messages (id, kind, payload_json, status, attempts,
         next_attempt_at, created_at, updated_at, sequence, conflict_group)
         VALUES (?, 'delete_project_artifact_dir', ?, 'pending', 0,
         strftime('%Y-%m-%dT%H:%M:%SZ', 'now'), '2026-01-01T00:00:00Z',
         '2026-01-01T00:00:00Z', ?, ?)",
    )
    .bind(message.id.to_string())
    .bind(serde_json::to_string(&message.payload)?)
    .bind(sequence)
    .bind(format!("project:{project}"))
    .execute(pool)
    .await?;
    Ok(())
}

#[tokio::test]
async fn outbox_independent_projects_enter_handlers_together() -> Result<(), Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    let pool = connect_sqlite(root.path().join("outbox.sqlite")).await?;
    insert(&pool, ProjectId::new()).await?;
    insert(&pool, ProjectId::new()).await?;
    let gate = Arc::new(Barrier::new(3));
    let entered = Arc::new(AtomicUsize::new(0));
    let store = BarrierStore {
        gate: gate.clone(),
        entered: entered.clone(),
        panic_on_delete: false,
        selective: None,
    };
    let worker = Arc::new(OutboxWorker::new(
        SqliteOutboxRepository::new(pool.clone()),
        store,
        Arc::new(SqliteArtifactIndex::new(pool.clone())),
        Arc::new(SqliteStorageUnitOfWork::new(pool.clone())),
        Arc::new(NoEvents),
        Arc::new(LocalTempWorkspace::new(root.path())),
        OutboxMaintenanceConfig::try_default()?,
    ));
    let task = tokio::spawn(async move { worker.process_pending_messages(2).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), gate.wait()).await?;
    assert_eq!(entered.load(Ordering::SeqCst), 2);
    let report = task.await??;
    assert_eq!(report.completed, 2);
    assert_eq!(report.max_active, 2);
    pool.close().await;
    Ok(())
}

#[tokio::test]
async fn released_slot_starts_another_group_while_one_group_is_slow() -> Result<(), Box<dyn Error>>
{
    let root = tempfile::tempdir()?;
    let pool = connect_sqlite(root.path().join("refill.sqlite")).await?;
    let slow = ProjectId::new();
    insert(&pool, slow.clone()).await?;
    insert(&pool, ProjectId::new()).await?;
    insert(&pool, ProjectId::new()).await?;
    let selective = Arc::new(SelectiveGate {
        slow_project: slow,
        release_slow: Notify::new(),
        fast_seen: Notify::new(),
        fast_count: AtomicUsize::new(0),
    });
    let store = BarrierStore {
        gate: Arc::new(Barrier::new(1)),
        entered: Arc::new(AtomicUsize::new(0)),
        panic_on_delete: false,
        selective: Some(selective.clone()),
    };
    let worker = OutboxWorker::new(
        SqliteOutboxRepository::new(pool.clone()),
        store,
        Arc::new(SqliteArtifactIndex::new(pool.clone())),
        Arc::new(SqliteStorageUnitOfWork::new(pool.clone())),
        Arc::new(NoEvents),
        Arc::new(LocalTempWorkspace::new(root.path())),
        OutboxMaintenanceConfig::try_default()?,
    );
    let task = tokio::spawn(async move { worker.process_pending_messages(3).await });
    while selective.fast_count.load(Ordering::SeqCst) < 2 {
        tokio::time::timeout(Duration::from_secs(5), selective.fast_seen.notified()).await?;
    }
    selective.release_slow.notify_one();
    let report = task.await??;
    assert_eq!(report.completed, 3);
    assert_eq!(report.max_active, 2);
    Ok(())
}

#[tokio::test]
async fn panicked_attempt_keeps_its_claim_recoverable() -> Result<(), Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    let pool = connect_sqlite(root.path().join("panic.sqlite")).await?;
    insert(&pool, ProjectId::new()).await?;
    let gate = Arc::new(Barrier::new(2));
    let store = BarrierStore {
        gate: gate.clone(),
        entered: Arc::new(AtomicUsize::new(0)),
        panic_on_delete: true,
        selective: None,
    };
    let repo = SqliteOutboxRepository::new(pool.clone());
    let worker = OutboxWorker::new(
        repo.clone(),
        store,
        Arc::new(SqliteArtifactIndex::new(pool.clone())),
        Arc::new(SqliteStorageUnitOfWork::new(pool.clone())),
        Arc::new(NoEvents),
        Arc::new(LocalTempWorkspace::new(root.path())),
        OutboxMaintenanceConfig::try_default()?,
    );
    let task = tokio::spawn(async move { worker.process_pending_messages(1).await });
    tokio::time::timeout(Duration::from_secs(5), gate.wait()).await?;
    assert!(matches!(task.await, Err(error) if error.is_panic()));
    let old_token: String =
        sqlx::query_scalar("SELECT locked_by FROM outbox_messages WHERE status = 'processing'")
            .fetch_one(&pool)
            .await?;
    sqlx::query("UPDATE outbox_messages SET locked_at = '2020-01-01T00:00:00Z'")
        .execute(&pool)
        .await?;
    let recovered = repo.claim_ready(1, Duration::from_secs(2)).await?;
    assert_eq!(recovered.claims.len(), 1);
    assert_ne!(recovered.claims[0].token, old_token);
    Ok(())
}
