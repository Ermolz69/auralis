#![allow(clippy::unwrap_used, clippy::expect_used)]
use domain::project::Project;
use ports::repository::ProjectRepository;

use crate::error::ApplicationError;

#[derive(Debug)]
pub struct CreateProjectRequest {
    pub title: String,
}

#[derive(Debug)]
pub struct CreateProjectResponse {
    pub project: Project,
}

pub struct CreateProjectUseCase<R: ProjectRepository> {
    project_repo: R,
}

impl<R: ProjectRepository> CreateProjectUseCase<R> {
    pub fn new(project_repo: R) -> Self {
        Self { project_repo }
    }

    pub async fn execute(
        &self,
        request: CreateProjectRequest,
    ) -> Result<CreateProjectResponse, ApplicationError> {
        let title = request.title.trim();
        if title.is_empty() {
            return Err(domain::error::DomainError::ValidationError(
                "Project title cannot be empty".to_string(),
            )
            .into());
        }
        let project = Project::new(title.to_string());
        let created_project = self.project_repo.create(project).await?;

        Ok(CreateProjectResponse {
            project: created_project,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapters_storage::memory::InMemoryProjectRepository;
    use domain::project::ProjectStatus;

    #[tokio::test]
    async fn test_create_project_success() {
        let repo = InMemoryProjectRepository::new(std::sync::Arc::new(std::sync::Mutex::new(
            adapters_storage::memory::InMemoryDatabase::new(),
        )));
        let use_case = CreateProjectUseCase::new(repo.clone());

        let request = CreateProjectRequest {
            title: "Test Project".to_string(),
        };

        let response = use_case.execute(request).await.unwrap();

        assert_eq!(response.project.title(), "Test Project");
        assert_eq!(*response.project.status(), ProjectStatus::Draft);

        // Verify it was saved
        let saved = repo.get(response.project.id()).await.unwrap().unwrap();
        assert_eq!(saved.id(), response.project.id());
    }

    #[tokio::test]
    async fn rejects_empty_project_title() {
        let repo = InMemoryProjectRepository::new(std::sync::Arc::new(std::sync::Mutex::new(
            adapters_storage::memory::InMemoryDatabase::new(),
        )));
        let use_case = CreateProjectUseCase::new(repo);

        let result = use_case
            .execute(CreateProjectRequest {
                title: "   ".to_string(),
            })
            .await;

        assert!(matches!(result, Err(ApplicationError::Domain(_))));
    }
}
