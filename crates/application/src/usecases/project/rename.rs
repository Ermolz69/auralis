use domain::project::{Project, ProjectId};
use ports::repository::ProjectRepository;

use crate::error::ApplicationError;

#[derive(Debug)]
pub struct RenameProjectRequest {
    pub project_id: ProjectId,
    pub title: String,
}

pub struct RenameProjectUseCase<R: ProjectRepository> {
    project_repo: R,
}

impl<R: ProjectRepository> RenameProjectUseCase<R> {
    pub fn new(project_repo: R) -> Self {
        Self { project_repo }
    }

    pub async fn execute(
        &self,
        request: RenameProjectRequest,
    ) -> Result<Project, ApplicationError> {
        let mut project = self
            .project_repo
            .get(&request.project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(request.project_id.clone()))?;
        project.set_title(request.title)?;
        self.project_repo.save(&project).await?;
        Ok(project)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use domain::error::DomainError;
    use ports::error::PortError;

    use super::*;

    #[derive(Clone, Copy, Default)]
    enum RepositoryFailure {
        #[default]
        None,
        Get,
        Save,
    }

    struct TestProjectRepository {
        project: Option<Project>,
        failure: RepositoryFailure,
        saved: Arc<Mutex<Vec<Project>>>,
    }

    impl TestProjectRepository {
        fn containing(project: Project) -> Self {
            Self {
                project: Some(project),
                failure: RepositoryFailure::None,
                saved: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn empty() -> Self {
            Self {
                project: None,
                failure: RepositoryFailure::None,
                saved: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn failing(failure: RepositoryFailure, project: Option<Project>) -> Self {
            Self {
                project,
                failure,
                saved: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn error(operation: &'static str) -> PortError {
            PortError::Storage {
                operation,
                message: format!("{operation} failed"),
            }
        }
    }

    #[async_trait]
    impl ProjectRepository for TestProjectRepository {
        async fn create(&self, _project: Project) -> Result<Project, PortError> {
            Err(Self::error("create"))
        }

        async fn get(&self, id: &ProjectId) -> Result<Option<Project>, PortError> {
            if matches!(self.failure, RepositoryFailure::Get) {
                return Err(Self::error("get"));
            }
            Ok(self
                .project
                .as_ref()
                .filter(|project| project.id() == id)
                .cloned())
        }

        async fn save(&self, project: &Project) -> Result<(), PortError> {
            if matches!(self.failure, RepositoryFailure::Save) {
                return Err(Self::error("save"));
            }
            self.saved.lock().unwrap().push(project.clone());
            Ok(())
        }

        async fn list(&self) -> Result<Vec<Project>, PortError> {
            Err(Self::error("list"))
        }

        async fn delete(&self, _id: &ProjectId) -> Result<(), PortError> {
            Err(Self::error("delete"))
        }
    }

    #[tokio::test]
    async fn renames_and_saves_an_existing_project() {
        let project = Project::new("Original title".to_string());
        let project_id = project.id().clone();
        let repository = TestProjectRepository::containing(project);
        let saved = repository.saved.clone();
        let use_case = RenameProjectUseCase::new(repository);

        let renamed = use_case
            .execute(RenameProjectRequest {
                project_id: project_id.clone(),
                title: "  Renamed project  ".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(renamed.id(), &project_id);
        assert_eq!(renamed.title(), "Renamed project");
        let saved = saved.lock().unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0], renamed);
    }

    #[tokio::test]
    async fn rejects_an_empty_title_without_saving() {
        let project = Project::new("Original title".to_string());
        let project_id = project.id().clone();
        let repository = TestProjectRepository::containing(project);
        let saved = repository.saved.clone();
        let use_case = RenameProjectUseCase::new(repository);

        let result = use_case
            .execute(RenameProjectRequest {
                project_id,
                title: "   ".to_string(),
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::Domain(DomainError::ValidationError(message)))
                if message == "Project title cannot be empty"
        ));
        assert!(saved.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn reports_a_missing_project_without_saving() {
        let missing_id = ProjectId::new();
        let repository = TestProjectRepository::empty();
        let saved = repository.saved.clone();
        let use_case = RenameProjectUseCase::new(repository);

        let result = use_case
            .execute(RenameProjectRequest {
                project_id: missing_id.clone(),
                title: "Renamed project".to_string(),
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::ProjectNotFound(id)) if id == missing_id
        ));
        assert!(saved.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn propagates_repository_get_failures() {
        let project_id = ProjectId::new();
        let repository = TestProjectRepository::failing(RepositoryFailure::Get, None);
        let saved = repository.saved.clone();
        let use_case = RenameProjectUseCase::new(repository);

        let result = use_case
            .execute(RenameProjectRequest {
                project_id,
                title: "Renamed project".to_string(),
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::Port(PortError::Storage {
                operation: "get",
                ..
            }))
        ));
        assert!(saved.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn propagates_repository_save_failures() {
        let project = Project::new("Original title".to_string());
        let project_id = project.id().clone();
        let repository = TestProjectRepository::failing(RepositoryFailure::Save, Some(project));
        let saved = repository.saved.clone();
        let use_case = RenameProjectUseCase::new(repository);

        let result = use_case
            .execute(RenameProjectRequest {
                project_id,
                title: "Renamed project".to_string(),
            })
            .await;

        assert!(matches!(
            result,
            Err(ApplicationError::Port(PortError::Storage {
                operation: "save",
                ..
            }))
        ));
        assert!(saved.lock().unwrap().is_empty());
    }
}
