use crate::error::ApplicationError;
use domain::project::{Project, ProjectId};
use ports::repository::ProjectRepository;

pub struct GetProjectRequest {
    pub project_id: ProjectId,
}

pub struct GetProjectResponse {
    pub project: Project,
}

pub struct GetProjectUseCase<R: ProjectRepository> {
    project_repo: R,
}

impl<R: ProjectRepository> GetProjectUseCase<R> {
    pub fn new(project_repo: R) -> Self {
        Self { project_repo }
    }

    pub async fn execute(
        &self,
        req: GetProjectRequest,
    ) -> Result<GetProjectResponse, ApplicationError> {
        let project = self
            .project_repo
            .get(&req.project_id)
            .await?
            .ok_or(ApplicationError::ProjectNotFound(req.project_id))?;

        Ok(GetProjectResponse { project })
    }
}
