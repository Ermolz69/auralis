#![allow(clippy::unwrap_used)]

use super::{
    delete::DeleteProjectUseCase, lifecycle::ProjectLifecycleLocks,
    youtube_storage_support::UnusedEventPublisher,
};
use crate::worker::outbox::{OutboxWorker, maintenance::OutboxMaintenanceConfig};
use adapters_storage::{
    local::{LocalArtifactStore, LocalTempWorkspace},
    sqlite::{
        SqliteArtifactIndex, SqliteOutboxRepository, SqliteProjectRepository,
        SqliteStorageUnitOfWork, connect_sqlite,
    },
};
use async_trait::async_trait;
use domain::project::{Project, ProjectId};
use ports::{
    error::PortError, project_workspace::ProjectWorkspacePort, repository::ProjectRepository,
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::sync::Notify;

#[derive(Default)]
pub(super) struct Gate {
    pub entered: Notify,
    pub released: Notify,
}

impl Gate {
    pub async fn wait_for_entry(&self) {
        tokio::time::timeout(Duration::from_secs(10), self.entered.notified())
            .await
            .unwrap();
    }
    async fn pause(&self) {
        self.entered.notify_one();
        tokio::time::timeout(Duration::from_secs(10), self.released.notified())
            .await
            .unwrap();
    }
}

pub(super) struct Workspace {
    pub root: PathBuf,
    pub gate: Option<Arc<Gate>>,
    pub calls: AtomicUsize,
    pub fail: bool,
}

#[async_trait]
impl ProjectWorkspacePort for Workspace {
    async fn ensure_and_open(&self, project_id: &ProjectId) -> Result<(), PortError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(gate) = &self.gate {
            gate.pause().await;
        }
        if self.fail {
            return Err(PortError::Io {
                message: "Injected workspace failure".into(),
            });
        }
        tokio::fs::create_dir_all(self.root.join(project_id.to_string()))
            .await
            .unwrap();
        Ok(())
    }
}

pub(super) struct ObservedRepository {
    pub inner: Arc<SqliteProjectRepository>,
    pub reads: AtomicUsize,
    pub fail: bool,
}

#[async_trait]
impl ProjectRepository for ObservedRepository {
    async fn get(&self, id: &ProjectId) -> Result<Option<Project>, PortError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            return Err(PortError::Storage {
                operation: "get_project",
                message: "Injected read failure".into(),
            });
        }
        self.inner.get(id).await
    }
    async fn create(&self, _project: Project) -> Result<Project, PortError> {
        unreachable!()
    }
    async fn update(
        &self,
        _id: &ProjectId,
        _expected_revision: u64,
        _update: ports::project_update::ProjectUpdate,
        _updated_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Project, PortError> {
        unreachable!()
    }
    async fn list(&self) -> Result<Vec<Project>, PortError> {
        unreachable!()
    }
    async fn delete(&self, _id: &ProjectId) -> Result<(), PortError> {
        unreachable!()
    }
}

struct Runtime {
    gate: Option<Arc<Gate>>,
}

#[async_trait]
impl ports::job_runtime_control::JobRuntimeControlPort for Runtime {
    async fn reserve(
        &self,
        _job_id: domain::job::JobId,
        _project_id: ProjectId,
    ) -> Result<(), PortError> {
        unreachable!()
    }
    async fn attach_task(
        &self,
        _job_id: domain::job::JobId,
        _task: ports::job_runtime_control::RuntimeTask,
    ) -> Result<(), ports::job_runtime_control::AttachTaskError> {
        unreachable!()
    }
    fn finish_now(&self, _job_id: &domain::job::JobId) {
        unreachable!()
    }
    async fn rollback_runtime_start(
        &self,
        _job_id: &domain::job::JobId,
    ) -> Result<ports::job_runtime_control::RuntimeCleanupOutcome, PortError> {
        unreachable!()
    }
    async fn cancel_and_evict_jobs(
        &self,
        job_ids: &[domain::job::JobId],
    ) -> Result<ports::job_runtime_control::RuntimeCleanupReport, PortError> {
        assert!(job_ids.is_empty());
        if let Some(gate) = &self.gate {
            gate.pause().await;
        }
        Ok(ports::job_runtime_control::RuntimeCleanupReport {
            jobs: Default::default(),
        })
    }
}

pub(super) struct Fixture {
    pub root: tempfile::TempDir,
    pub pool: sqlx::SqlitePool,
    pub repo: Arc<SqliteProjectRepository>,
    pub locks: Arc<ProjectLifecycleLocks>,
    pub project: Project,
}

impl Fixture {
    pub async fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let pool = connect_sqlite(root.path().join("test.db")).await.unwrap();
        let repo = Arc::new(SqliteProjectRepository::new(pool.clone()));
        let project = repo
            .create(Project::new("Workspace".into()).unwrap())
            .await
            .unwrap();
        Self {
            root,
            pool,
            repo,
            project,
            locks: Arc::new(ProjectLifecycleLocks::new()),
        }
    }
    pub fn workspace(&self, gate: Option<Arc<Gate>>, fail: bool) -> Arc<Workspace> {
        Arc::new(Workspace {
            root: self.root.path().join("projects"),
            gate,
            calls: AtomicUsize::new(0),
            fail,
        })
    }
    pub fn observed_repo(&self, fail: bool) -> Arc<ObservedRepository> {
        Arc::new(ObservedRepository {
            inner: self.repo.clone(),
            reads: AtomicUsize::new(0),
            fail,
        })
    }
    pub fn delete(&self, gate: Option<Arc<Gate>>) -> DeleteProjectUseCase {
        DeleteProjectUseCase::new(
            Arc::new(SqliteStorageUnitOfWork::new(self.pool.clone())),
            Arc::new(Runtime { gate }),
            self.locks.clone(),
        )
    }
    pub fn project_path(&self) -> PathBuf {
        self.root
            .path()
            .join("projects")
            .join(self.project.id().to_string())
    }
    pub async fn process_cleanup(&self) {
        let worker = OutboxWorker::new(
            SqliteOutboxRepository::new(self.pool.clone()),
            Arc::new(LocalArtifactStore::new(self.root.path().join("projects"))),
            Arc::new(SqliteArtifactIndex::new(self.pool.clone())),
            Arc::new(SqliteStorageUnitOfWork::new(self.pool.clone())),
            Arc::new(UnusedEventPublisher),
            Arc::new(LocalTempWorkspace::new(self.root.path().join("workspaces"))),
            OutboxMaintenanceConfig::try_default().unwrap(),
        );
        let report = worker.process_pending_messages(20).await.unwrap();
        assert!(report.completed > 0);
        assert_eq!(report.retry_scheduled, 0);
        assert_eq!(report.storage_errors, 0);
        assert!(!self.project_path().exists());
    }
}
