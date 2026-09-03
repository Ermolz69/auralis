use async_trait::async_trait;
use domain::project::{Project, ProjectId};
use ports::{error::PortError, project_update::ProjectUpdate, repository::ProjectRepository};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::Notify;

pub(super) struct PausedProjectRepository {
    inner: Arc<dyn ProjectRepository>,
    pause_on_ready: bool,
    armed: AtomicBool,
    reached: Notify,
    released: Notify,
}

impl PausedProjectRepository {
    pub(super) fn new(inner: Arc<dyn ProjectRepository>, pause_on_ready: bool) -> Arc<Self> {
        Arc::new(Self {
            inner,
            pause_on_ready,
            armed: AtomicBool::new(true),
            reached: Notify::new(),
            released: Notify::new(),
        })
    }

    pub(super) async fn wait_for_write(&self) {
        tokio::time::timeout(Duration::from_secs(10), self.reached.notified())
            .await
            .unwrap();
    }

    pub(super) fn release(&self) {
        self.released.notify_one();
    }
}

#[async_trait]
impl ProjectRepository for PausedProjectRepository {
    async fn create(&self, project: Project) -> Result<Project, PortError> {
        self.inner.create(project).await
    }

    async fn get(&self, id: &ProjectId) -> Result<Option<Project>, PortError> {
        self.inner.get(id).await
    }

    async fn update(
        &self,
        id: &ProjectId,
        revision: u64,
        update: ProjectUpdate,
        updated_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<Project, PortError> {
        if (!self.pause_on_ready || matches!(update, ProjectUpdate::MarkReadyForProcessing))
            && self.armed.swap(false, Ordering::SeqCst)
        {
            self.reached.notify_one();
            tokio::time::timeout(Duration::from_secs(10), self.released.notified())
                .await
                .unwrap();
        }
        self.inner.update(id, revision, update, updated_at).await
    }

    async fn list(&self) -> Result<Vec<Project>, PortError> {
        self.inner.list().await
    }
    async fn delete(&self, id: &ProjectId) -> Result<(), PortError> {
        self.inner.delete(id).await
    }
}
