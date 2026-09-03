#![allow(clippy::unwrap_used)]

use super::lifecycle::ProjectLifecycleLocks;
use super::open_folder::*;
use crate::error::ApplicationError;
use adapters_storage::memory::{InMemoryDatabase, InMemoryProjectRepository};
use async_trait::async_trait;
use domain::project::{Project, ProjectId};
use ports::error::PortError;
use ports::project_workspace::ProjectWorkspacePort;
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
    let project = Project::new("Workspace".to_string()).unwrap();
    repo.create(project.clone()).await.unwrap();
    let workspace = RecordingWorkspace::default();
    let opened = workspace.opened.clone();
    let use_case =
        OpenProjectFolderUseCase::new(repo, workspace, Arc::new(ProjectLifecycleLocks::new()));

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
    let use_case =
        OpenProjectFolderUseCase::new(repo, workspace, Arc::new(ProjectLifecycleLocks::new()));

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
