use std::error::Error;
use std::sync::Arc;
use std::time::{Duration, Instant};

use adapters_storage::{
    local::{LocalArtifactStore, LocalTempWorkspace},
    sqlite::{
        SqliteArtifactIndex, SqliteOutboxRepository, SqliteProjectRepository,
        SqliteStorageUnitOfWork, connect_sqlite,
    },
};
use application::worker::outbox::{OutboxWorker, maintenance::OutboxMaintenanceConfig};
use domain::{media::ArtifactKind, project::Project};
use ports::{
    outbox_resources::OutboxResourceGate,
    repository::ProjectRepository,
    storage::ArtifactStore,
    transaction::{CommitProjectDelete, CommitStagedArtifactWrite, StorageUnitOfWork},
};
use tokio::sync::Semaphore;

struct DiskGate(Arc<Semaphore>);

impl OutboxResourceGate for DiskGate {
    fn try_acquire_disk(&self) -> Option<Arc<dyn Send + Sync>> {
        self.0
            .clone()
            .try_acquire_owned()
            .ok()
            .map(|permit| Arc::new(permit) as Arc<dyn Send + Sync>)
    }
}

struct NoEvents;

#[async_trait::async_trait]
impl ports::events::AppEventPublisher for NoEvents {
    async fn publish_project_updated(&self, _: &str) -> Result<(), ports::PortError> {
        Ok(())
    }
    async fn publish_transcript_ready(&self, _: &str, _: &str) -> Result<(), ports::PortError> {
        Ok(())
    }
}

async fn run(concurrency: usize) -> Result<(Duration, usize), Box<dyn Error>> {
    const PROJECTS: usize = 8;
    let root = tempfile::tempdir()?;
    let pool = connect_sqlite(root.path().join("bench.sqlite")).await?;
    let projects = SqliteProjectRepository::new(pool.clone());
    let store = Arc::new(LocalArtifactStore::new(root.path().join("artifacts")));
    let uow = Arc::new(SqliteStorageUnitOfWork::new(pool.clone()));
    let source = root.path().join("source.mp4");
    std::fs::write(&source, vec![0xA5; 2 * 1024 * 1024])?;
    for index in 0..PROJECTS {
        let project = projects
            .create(Project::new(format!("Benchmark {index}"))?)
            .await?;
        let staged = store
            .import_external_file(project.id(), ArtifactKind::SourceVideo, &source, None)
            .await?;
        uow.commit_staged_artifact_write(CommitStagedArtifactWrite {
            project_id: project.id().clone(),
            artifact: staged.artifact,
            staging_key: staged.staging_key,
            final_key: staged.final_key,
            temp_workspace_key: None,
        })
        .await?;
    }
    let mut config = OutboxMaintenanceConfig::try_default()?;
    config.max_active = concurrency;
    let worker = OutboxWorker::new(
        SqliteOutboxRepository::new(pool.clone()),
        store,
        Arc::new(SqliteArtifactIndex::new(pool.clone())),
        uow,
        Arc::new(NoEvents),
        Arc::new(LocalTempWorkspace::new(root.path())),
        config,
    )
    .with_resource_gate(Arc::new(DiskGate(Arc::new(Semaphore::new(2)))));
    let start = Instant::now();
    let report = worker.process_pending_messages(PROJECTS).await?;
    let elapsed = start.elapsed();
    let failures: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT kind, status, last_error FROM outbox_messages WHERE status != 'done'",
    )
    .fetch_all(&pool)
    .await?;
    assert_eq!(report.completed, PROJECTS, "{report:?}; {failures:?}");
    assert!(report.max_active <= concurrency);
    let ready: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM artifacts WHERE state = 'ready'")
        .fetch_one(&pool)
        .await?;
    assert_eq!(ready, PROJECTS as i64);
    let pending: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM outbox_messages WHERE status != 'done'")
            .fetch_one(&pool)
            .await?;
    assert_eq!(pending, 0);
    pool.close().await;
    Ok((elapsed, report.max_active))
}

#[tokio::test]
#[ignore = "local disk comparison is reported separately from deterministic correctness tests"]
async fn compare_real_local_publication_under_the_same_disk_budget() -> Result<(), Box<dyn Error>> {
    let mut sequential = Vec::new();
    let mut parallel = Vec::new();
    for _ in 0..3 {
        sequential.push(run(1).await?);
        parallel.push(run(2).await?);
    }
    println!(
        "sequential: {sequential:?}; parallel: {parallel:?}; disk budget: 2; projects: 8; file size: 2 MiB"
    );
    Ok(())
}

#[tokio::test]
async fn outbox_real_artifacts_publish_then_cleanup() -> Result<(), Box<dyn Error>> {
    let root = tempfile::tempdir()?;
    let pool = connect_sqlite(root.path().join("e2e.sqlite")).await?;
    let projects = SqliteProjectRepository::new(pool.clone());
    let store = Arc::new(LocalArtifactStore::new(root.path().join("artifacts")));
    let uow = Arc::new(SqliteStorageUnitOfWork::new(pool.clone()));
    let source = root.path().join("source.mp4");
    std::fs::write(&source, b"multiple real artifacts")?;
    let mut saved = Vec::new();
    for index in 0..3 {
        let project = projects
            .create(Project::new(format!("End to end {index}"))?)
            .await?;
        let staged = store
            .import_external_file(project.id(), ArtifactKind::SourceVideo, &source, None)
            .await?;
        uow.commit_staged_artifact_write(CommitStagedArtifactWrite {
            project_id: project.id().clone(),
            artifact: staged.artifact,
            staging_key: staged.staging_key,
            final_key: staged.final_key.clone(),
            temp_workspace_key: None,
        })
        .await?;
        saved.push((project.id().clone(), staged.final_key));
    }
    let worker = OutboxWorker::new(
        SqliteOutboxRepository::new(pool.clone()),
        store.clone(),
        Arc::new(SqliteArtifactIndex::new(pool.clone())),
        uow.clone(),
        Arc::new(NoEvents),
        Arc::new(LocalTempWorkspace::new(root.path())),
        OutboxMaintenanceConfig::try_default()?,
    );
    let publication_report = worker.process_pending_messages(3).await?;
    let failures: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT kind, status, last_error FROM outbox_messages WHERE status != 'done'",
    )
    .fetch_all(&pool)
    .await?;
    assert_eq!(
        publication_report.completed, 3,
        "{publication_report:?}; {failures:?}"
    );
    for (_, key) in &saved {
        assert_eq!(
            std::fs::read(root.path().join("artifacts").join(key))?,
            b"multiple real artifacts"
        );
    }
    for (project_id, _) in &saved {
        uow.commit_project_delete(CommitProjectDelete {
            project_id: project_id.clone(),
        })
        .await?;
    }
    assert_eq!(worker.process_pending_messages(20).await?.completed, 6);
    for (project_id, key) in &saved {
        assert!(!root.path().join("artifacts").join(key).exists());
        assert!(
            !root
                .path()
                .join("artifacts")
                .join(project_id.to_string())
                .exists()
        );
    }
    let pending: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM outbox_messages WHERE status != 'done'")
            .fetch_one(&pool)
            .await?;
    assert_eq!(pending, 0);
    pool.close().await;
    Ok(())
}
