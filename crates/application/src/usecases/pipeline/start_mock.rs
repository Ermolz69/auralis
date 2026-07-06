use domain::project::{Project, ProjectId};
use ports::repository::ProjectRepository;
use jobs::manager::JobManager;
use jobs::job::Job as JobDto;

use crate::error::ApplicationError;

#[derive(Debug)]
pub struct StartMockPipelineRequest {
    pub project_id: ProjectId,
}

#[derive(Debug)]
pub struct StartMockPipelineResponse {
    pub project: Project,
    pub job: JobDto,
}

pub struct StartMockPipelineUseCase<R: ProjectRepository> {
    project_repo: R,
    job_manager: JobManager,
}

impl<R: ProjectRepository> StartMockPipelineUseCase<R> {
    pub fn new(project_repo: R, job_manager: JobManager) -> Self {
        Self {
            project_repo,
            job_manager,
        }
    }

    pub async fn execute(
        &self,
        request: StartMockPipelineRequest,
    ) -> Result<StartMockPipelineResponse, ApplicationError> {
        let mut project = self
            .project_repo
            .get(&request.project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(request.project_id.clone()))?;

        project.mark_processing_started()?;
        self.project_repo.save(&project).await?;

        // Launch job asynchronously via JobManager
        let job_id = self.job_manager.start_mock_dubbing_job(project.title().to_string(), Some(project.id().to_string())).await;
        let job = self.job_manager.get_job(&job_id).await.unwrap();

        Ok(StartMockPipelineResponse { project, job })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use adapters_storage::memory::InMemoryProjectRepository;
    use domain::project::ProjectStatus;
    use jobs::status::JobStatus;

    #[tokio::test]
    async fn test_start_mock_pipeline_success() {
        let project_repo = InMemoryProjectRepository::new();
        let job_manager = JobManager::new(None);
        
        let mut project = Project::new("Test".to_string());
        let source = domain::media::MediaSource::RemoteUrl { url: "http://example.com".to_string() };
        project.import_source(source, None).unwrap();
        project.mark_ready_for_processing().unwrap();

        project_repo.create(project.clone()).await.unwrap();

        let use_case = StartMockPipelineUseCase::new(project_repo.clone(), job_manager.clone());
        
        let request = StartMockPipelineRequest {
            project_id: project.id().clone(),
        };

        let response = use_case.execute(request).await.unwrap();
        
        // Ensure job is enqueued or running
        assert!(response.job.status == JobStatus::Queued || response.job.status == JobStatus::Running);
    }
}
