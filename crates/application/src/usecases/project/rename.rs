use domain::project::{Project, ProjectId};
use ports::project_update::ProjectUpdate;
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
        Ok(self
            .project_repo
            .update(
                project.id(),
                project.revision(),
                ProjectUpdate::Rename {
                    title: project.title().to_string(),
                },
                project.updated_at(),
            )
            .await?)
    }
}
