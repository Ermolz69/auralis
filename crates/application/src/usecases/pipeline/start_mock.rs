use domain::project::{Project, ProjectId};
use ports::job_scheduler::{JobSchedulerPort, ScheduledJob, StartDubbingJobRequest};
use ports::repository::ProjectRepository;
use std::sync::Arc;

use crate::error::ApplicationError;

#[derive(Debug)]
pub struct StartMockPipelineRequest {
    pub project_id: ProjectId,
}

#[derive(Debug)]
pub struct StartMockPipelineResponse {
    pub project: Project,
    pub job: ScheduledJob,
}

pub struct StartMockPipelineUseCase<R: ProjectRepository> {
    project_repo: R,
    job_scheduler: Arc<dyn JobSchedulerPort>,
}

impl<R: ProjectRepository> StartMockPipelineUseCase<R> {
    pub fn new(project_repo: R, job_scheduler: Arc<dyn JobSchedulerPort>) -> Self {
        Self {
            project_repo,
            job_scheduler,
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

        // Launch job asynchronously via JobSchedulerPort
        let job = self
            .job_scheduler
            .start_dubbing_job(StartDubbingJobRequest {
                project_id: Some(project.id().clone()),
                title: project.title().to_string(),
            })
            .await
            .map_err(|e| ApplicationError::InvalidOperation {
                message: format!("Failed to start job: {}", e),
            })?;

        Ok(StartMockPipelineResponse { project, job })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockJobScheduler;
    use adapters_storage::memory::InMemoryProjectRepository;
    use domain::job::JobStatus;

    #[tokio::test]
    async fn test_start_mock_pipeline_success() {
        let project_repo = InMemoryProjectRepository::new();
        let job_scheduler = Arc::new(MockJobScheduler::new());

        let mut project = Project::new("Test".to_string());
        let source = domain::media::MediaSource::RemoteUrl {
            url: "http://example.com".to_string(),
        };
        project.import_source(source, None).unwrap();
        project.mark_ready_for_processing().unwrap();

        project_repo.create(project.clone()).await.unwrap();

        let use_case = StartMockPipelineUseCase::new(project_repo.clone(), job_scheduler.clone());

        let request = StartMockPipelineRequest {
            project_id: project.id().clone(),
        };

        let response = use_case.execute(request).await.unwrap();

        // Ensure job is enqueued or pending
        assert!(
            response.job.status == JobStatus::Pending || response.job.status == JobStatus::Running
        );
    }
}
