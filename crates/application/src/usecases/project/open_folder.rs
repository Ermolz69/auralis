use crate::error::ApplicationError;
use domain::project::ProjectId;
use ports::project_workspace::ProjectWorkspacePort;
use ports::repository::ProjectRepository;

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
}

impl<R, W> OpenProjectFolderUseCase<R, W>
where
    R: ProjectRepository,
    W: ProjectWorkspacePort,
{
    pub fn new(project_repo: R, workspace: W) -> Self {
        Self {
            project_repo,
            workspace,
        }
    }

    pub async fn execute(&self, request: OpenProjectFolderRequest) -> Result<(), ApplicationError> {
        self.project_repo
            .get(&request.project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(request.project_id.clone()))?;

        self.workspace.ensure_and_open(&request.project_id).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapters_storage::memory::{InMemoryDatabase, InMemoryProjectRepository};
    use async_trait::async_trait;
    use domain::project::Project;
    use ports::error::PortError;
    use ports::repository::ProjectRepository;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct RecordingWorkspace {
        opened: Arc<Mutex<Vec<ProjectId>>>,
    }

    #[async_trait]
    impl ProjectWorkspacePort for RecordingWorkspace {
        async fn ensure_and_open(&self, project_id: &ProjectId) -> Result<(), PortError> {
            self.opened
                .lock()
                .map_err(|_| PortError::Unexpected {
                    message: "Recording workspace lock poisoned".to_string(),
                })?
                .push(project_id.clone());
            Ok(())
        }
    }

    fn repository() -> InMemoryProjectRepository {
        InMemoryProjectRepository::new(Arc::new(Mutex::new(InMemoryDatabase::new())))
    }

    #[tokio::test]
    async fn existing_project_workspace_is_opened() {
        let repo = repository();
        let project = Project::new("Workspace".to_string());
        repo.create(project.clone()).await.unwrap();
        let workspace = RecordingWorkspace::default();
        let opened = workspace.opened.clone();
        let use_case = OpenProjectFolderUseCase::new(repo, workspace);

        use_case
            .execute(OpenProjectFolderRequest {
                project_id: project.id().clone(),
            })
            .await
            .unwrap();

        assert_eq!(*opened.lock().unwrap(), vec![project.id().clone()]);
    }

    #[tokio::test]
    async fn missing_project_does_not_touch_the_workspace() {
        let repo = repository();
        let workspace = RecordingWorkspace::default();
        let opened = workspace.opened.clone();
        let missing_id = ProjectId::new();
        let use_case = OpenProjectFolderUseCase::new(repo, workspace);

        let result = use_case
            .execute(OpenProjectFolderRequest {
                project_id: missing_id.clone(),
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::ProjectNotFound(id)) if id == missing_id
        ));
        assert!(opened.lock().unwrap().is_empty());
    }
}
