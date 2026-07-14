use crate::error::ApplicationError;
use ports::repository::ProjectRepository;
use std::str::FromStr;

pub struct HandleJobCompletedRequest {
    pub job_id: String,
    pub project_id: String,
    pub is_success: bool,
}

pub struct HandleJobCompletedResult {
    pub transcript_ready: bool,
}

pub struct HandleJobCompletedUseCase<R: ProjectRepository + Clone + 'static> {
    project_repo: R,
}

impl<R: ProjectRepository + Clone + 'static> HandleJobCompletedUseCase<R> {
    pub fn new(project_repo: R) -> Self {
        Self { project_repo }
    }

    pub async fn execute(
        &self,
        req: HandleJobCompletedRequest,
    ) -> Result<HandleJobCompletedResult, ApplicationError> {
        let pid = domain::project::ProjectId::from_str(&req.project_id).map_err(|e| {
            ApplicationError::InvalidOperation {
                message: e.to_string(),
            }
        })?;

        let project = self
            .project_repo
            .get(&pid)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(pid.clone()))?;

        let transcript_ready = project.transcript().is_some();

        Ok(HandleJobCompletedResult { transcript_ready })
    }
}
