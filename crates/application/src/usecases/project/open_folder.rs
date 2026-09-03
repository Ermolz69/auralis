use crate::error::ApplicationError;
use domain::project::ProjectId;
use ports::project_workspace::ProjectWorkspacePort;
use ports::repository::ProjectRepository;
use std::sync::Arc;

use super::lifecycle::ProjectLifecycleLocks;

pub struct OpenProjectFolderRequest {
    pub project_id: ProjectId,
}

pub struct OpenProjectFolderUseCase<R, W>
where
    R: ProjectRepository,
    W: ProjectWorkspacePort,
{
    project_repo: R,
    workspace: W,
    locks: Arc<ProjectLifecycleLocks>,
}

impl<R, W> OpenProjectFolderUseCase<R, W>
where
    R: ProjectRepository,
    W: ProjectWorkspacePort,
{
    pub fn new(project_repo: R, workspace: W, locks: Arc<ProjectLifecycleLocks>) -> Self {
        Self {
            project_repo,
            workspace,
            locks,
        }
    }

    pub async fn execute(&self, request: OpenProjectFolderRequest) -> Result<(), ApplicationError> {
        let lock = self.locks.get_lock(&request.project_id)?;
        let _guard = lock.lock().await;
        self.project_repo
            .get(&request.project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(request.project_id.clone()))?;

        self.workspace.ensure_and_open(&request.project_id).await?;

        Ok(())
    }
}
