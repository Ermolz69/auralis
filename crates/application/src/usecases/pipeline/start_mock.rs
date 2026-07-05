use domain::dubbing::DubbingPipelineStage;
use domain::job::{Job, JobKind, JobProgress};
use domain::project::{Project, ProjectId};
use ports::repository::{JobRepository, ProjectRepository};

use crate::error::ApplicationError;

#[derive(Debug)]
pub struct StartMockPipelineRequest {
    pub project_id: ProjectId,
}

#[derive(Debug)]
pub struct StartMockPipelineResponse {
    pub project: Project,
    pub job: Job,
}

pub struct StartMockPipelineUseCase<R: ProjectRepository, J: JobRepository> {
    project_repo: R,
    job_repo: J,
}

impl<R: ProjectRepository, J: JobRepository> StartMockPipelineUseCase<R, J> {
    pub fn new(project_repo: R, job_repo: J) -> Self {
        Self {
            project_repo,
            job_repo,
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

        let job = Job::new(request.project_id.clone(), JobKind::Dubbing);
        let mut job = self.job_repo.create(job).await?;

        job.start()?;
        job.update_stage(DubbingPipelineStage::ValidateSource)?;
        job.update_progress(JobProgress {
            percent: 5,
            message: "Validating source...".to_string(),
            current_step: None,
            processed_items: None,
            total_items: None,
        })?;
        self.job_repo.save(&job).await?;

        job.update_stage(DubbingPipelineStage::FetchMetadata)?;
        job.update_progress(JobProgress {
            percent: 20,
            message: "Fetching metadata...".to_string(),
            current_step: None,
            processed_items: None,
            total_items: None,
        })?;
        self.job_repo.save(&job).await?;

        job.update_stage(DubbingPipelineStage::ExtractOrGenerateTranscript)?;
        job.update_progress(JobProgress {
            percent: 50,
            message: "Transcribing...".to_string(),
            current_step: None,
            processed_items: None,
            total_items: None,
        })?;
        self.job_repo.save(&job).await?;

        job.update_stage(DubbingPipelineStage::SynthesizeSegments)?;
        job.update_progress(JobProgress {
            percent: 75,
            message: "Generating speech...".to_string(),
            current_step: None,
            processed_items: None,
            total_items: None,
        })?;
        self.job_repo.save(&job).await?;

        job.update_stage(DubbingPipelineStage::MuxAudioTrack)?;
        job.update_progress(JobProgress {
            percent: 90,
            message: "Muxing media...".to_string(),
            current_step: None,
            processed_items: None,
            total_items: None,
        })?;
        self.job_repo.save(&job).await?;

        job.mark_completed()?;
        self.job_repo.save(&job).await?;

        project.mark_completed()?;
        self.project_repo.save(&project).await?;

        Ok(StartMockPipelineResponse { project, job })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usecases::test_utils::mocks::{InMemoryJobRepository, InMemoryProjectRepository};
    use domain::job::JobStatus;
    use domain::project::ProjectStatus;

    #[tokio::test]
    async fn test_start_mock_pipeline_success() {
        let project_repo = InMemoryProjectRepository::new();
        let job_repo = InMemoryJobRepository::new();
        
        let mut project = Project::new("Test".to_string());
        let source = domain::media::MediaSource::RemoteUrl { url: "http://example.com".to_string() };
        project.import_source(source, None).unwrap();
        project.mark_ready_for_processing().unwrap();

        project_repo.create(project.clone()).await.unwrap();

        let use_case = StartMockPipelineUseCase::new(project_repo.clone(), job_repo.clone());
        
        let request = StartMockPipelineRequest {
            project_id: project.id().clone(),
        };

        let response = use_case.execute(request).await.unwrap();
        
        assert_eq!(*response.project.status(), ProjectStatus::Completed);
        assert_eq!(*response.job.status(), JobStatus::Completed);
        assert_eq!(response.job.progress().percent, 100);

        let saved_project = project_repo.get(project.id()).await.unwrap().unwrap();
        assert_eq!(*saved_project.status(), ProjectStatus::Completed);

        let saved_job = job_repo.get(response.job.id()).await.unwrap().unwrap();
        assert_eq!(*saved_job.status(), JobStatus::Completed);
        assert_eq!(saved_job.progress().percent, 100);
    }

    #[tokio::test]
    async fn test_start_mock_pipeline_not_found() {
        let project_repo = InMemoryProjectRepository::new();
        let job_repo = InMemoryJobRepository::new();
        let use_case = StartMockPipelineUseCase::new(project_repo, job_repo);
        
        let request = StartMockPipelineRequest {
            project_id: domain::project::ProjectId::new(),
        };

        let err = use_case.execute(request).await.unwrap_err();
        assert!(matches!(err, ApplicationError::ProjectNotFound(_)));
    }
}
