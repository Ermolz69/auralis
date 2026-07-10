use crate::error::ApplicationError;
use domain::project::ProjectId;
use domain::transcript::Transcript;
use ports::repository::ProjectRepository;

pub struct GetTranscriptRequest {
    pub project_id: ProjectId,
}

pub struct GetTranscriptResponse {
    pub transcript: Option<Transcript>,
}

pub struct GetTranscriptUseCase<R: ProjectRepository> {
    project_repo: R,
}

impl<R: ProjectRepository> GetTranscriptUseCase<R> {
    pub fn new(project_repo: R) -> Self {
        Self { project_repo }
    }

    pub async fn execute(
        &self,
        req: GetTranscriptRequest,
    ) -> Result<GetTranscriptResponse, ApplicationError> {
        let project = self
            .project_repo
            .get(&req.project_id)
            .await?
            .ok_or(ApplicationError::ProjectNotFound(req.project_id))?;

        let transcript = project.transcript().cloned();

        Ok(GetTranscriptResponse { transcript })
    }
}
