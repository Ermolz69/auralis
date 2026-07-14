use crate::error::ApplicationError;
use domain::project::{Project, ProjectId};
use ports::repository::ProjectRepository;
use std::str::FromStr;

#[derive(Debug)]
pub struct HandleJobCancelledRequest {
    pub job_id: String,
    pub project_id: String,
}

#[derive(Debug)]
pub struct HandleJobCancelledResult {
    pub project: Project,
    pub job_id: String,
}

pub struct HandleJobCancelledUseCase<R: ProjectRepository> {
    project_repo: R,
}

impl<R: ProjectRepository> HandleJobCancelledUseCase<R> {
    pub fn new(project_repo: R) -> Self {
        Self { project_repo }
    }

    pub async fn execute(
        &self,
        request: HandleJobCancelledRequest,
    ) -> Result<HandleJobCancelledResult, ApplicationError> {
        let project_id = ProjectId::from_str(&request.project_id).map_err(|error| {
            ApplicationError::InvalidOperation {
                message: format!("Invalid project id '{}': {}", request.project_id, error),
            }
        })?;

        let project = self
            .project_repo
            .get(&project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(project_id.clone()))?;

        Ok(HandleJobCancelledResult {
            project,
            job_id: request.job_id,
        })
    }
}
