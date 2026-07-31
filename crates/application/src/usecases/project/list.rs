use crate::error::ApplicationError;
use domain::project::Project;
use ports::repository::ProjectRepository;

pub struct ListProjectsRequest {}

pub struct ListProjectsResponse {
    pub projects: Vec<Project>,
}

pub struct ListProjectsUseCase<R: ProjectRepository> {
    project_repo: R,
}

impl<R: ProjectRepository> ListProjectsUseCase<R> {
    pub fn new(project_repo: R) -> Self {
        Self { project_repo }
    }

    pub async fn execute(
        &self,
        _req: ListProjectsRequest,
    ) -> Result<ListProjectsResponse, ApplicationError> {
        let projects = self.project_repo.list().await?;
        Ok(ListProjectsResponse { projects })
    }
}
