use crate::error::ApplicationError;
use ports::repository::ProjectRepository;
use std::str::FromStr;

pub struct HandleJobCancelledRequest {
    pub job_id: String,
    pub project_id: String,
}

pub struct HandleJobCancelledUseCase<R: ProjectRepository + Clone + 'static> {
    project_repo: R,
}

impl<R: ProjectRepository + Clone + 'static> HandleJobCancelledUseCase<R> {
    pub fn new(project_repo: R) -> Self {
        Self { project_repo }
    }

    pub async fn execute(&self, req: HandleJobCancelledRequest) -> Result<(), ApplicationError> {
        let pid = domain::project::ProjectId::from_str(&req.project_id).map_err(|e| {
            ApplicationError::InvalidOperation {
                message: e.to_string(),
            }
        })?;

        let mut project = self
            .project_repo
            .get(&pid)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(pid.clone()))?;

        project.cancel()?;

        self.project_repo.save(&project).await?;

        Ok(())
    }
}
