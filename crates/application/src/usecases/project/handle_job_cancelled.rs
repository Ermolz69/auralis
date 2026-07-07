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

        let mut project = self
            .project_repo
            .get(&project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(project_id.clone()))?;

        project.cancel()?;
        self.project_repo.save(&project).await?;

        Ok(HandleJobCancelledResult {
            project,
            job_id: request.job_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapters_storage::memory::InMemoryProjectRepository;
    use domain::media::MediaSource;
    use domain::project::{Project, ProjectStatus};
    use ports::repository::ProjectRepository;

    #[tokio::test]
    async fn cancelling_processing_project_marks_it_cancelled() {
        let project_repo = InMemoryProjectRepository::new();

        let mut project = Project::new("Cancellation test".to_string());
        project
            .import_source(
                MediaSource::RemoteUrl {
                    url: "https://example.com/video.mp4".to_string(),
                },
                None,
            )
            .unwrap();
        project.mark_ready_for_processing().unwrap();
        project.mark_processing_started().unwrap();

        let project_id = project.id().clone();
        project_repo.create(project).await.unwrap();

        let use_case = HandleJobCancelledUseCase::new(project_repo.clone());
        let result = use_case
            .execute(HandleJobCancelledRequest {
                job_id: "job-1".to_string(),
                project_id: project_id.to_string(),
            })
            .await
            .unwrap();

        assert_eq!(result.job_id, "job-1");
        assert_eq!(result.project.status(), &ProjectStatus::Cancelled);

        let saved_project = project_repo.get(&project_id).await.unwrap().unwrap();
        assert_eq!(saved_project.status(), &ProjectStatus::Cancelled);
    }
}
