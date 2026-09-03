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
        let project = Project::new(request.title)?;
        let created_project = self.project_repo.create(project).await?;

        Ok(CreateProjectResponse {
            project: created_project,
        })
    }
}
