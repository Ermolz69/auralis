use domain::project::{Project, ProjectId};
use ports::job_scheduler::{JobSchedulerPort, ScheduledJob};
use ports::repository::ProjectRepository;
use ports::transaction::{TransactionGateway, UnitOfWorkData};
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
    transaction_gateway: Arc<dyn TransactionGateway>,
}

impl<R: ProjectRepository> StartMockPipelineUseCase<R> {
    pub fn new(
        project_repo: R,
        job_scheduler: Arc<dyn JobSchedulerPort>,
        transaction_gateway: Arc<dyn TransactionGateway>,
    ) -> Self {
        Self {
            project_repo,
            job_scheduler,
            transaction_gateway,
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

        // 1. Validate transition
        project.mark_processing_started()?;

        // 2. Create Job (it starts as Pending)
        let job = domain::job::Job::new(
            project.id().clone(),
            project.title().to_string(),
            domain::job::JobKind::Dubbing,
        );

        let job_id = job.id().clone();

        // 3. Atomically persist Project and Job changes
        let uow = UnitOfWorkData::new()
            .save_project(project.clone())
            .save_job(job);

        self.transaction_gateway.execute(uow).await.map_err(|e| {
            ApplicationError::InvalidOperation {
                message: format!("Failed to commit transaction: {}", e),
            }
        })?;

        // 4. Enqueue the persisted job for asynchronous processing
        let job = self
            .job_scheduler
            .enqueue_existing_job(&job_id)
            .await
            .map_err(|e| ApplicationError::InvalidOperation {
                message: format!("Failed to enqueue existing job: {}", e),
            })?;

        Ok(StartMockPipelineResponse { project, job })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{MockJobScheduler, MockTransactionGateway};
    use adapters_storage::memory::InMemoryProjectRepository;
    use domain::job::JobStatus;

    #[tokio::test]
    async fn test_success_saves_project_processing_and_job_pending_running() {
        let project_repo = InMemoryProjectRepository::new();
        let job_scheduler = Arc::new(MockJobScheduler::new());
        let tx_gateway = Arc::new(MockTransactionGateway::new());

        let mut project = Project::new("Test".to_string());
        project
            .import_source(
                domain::media::MediaSource::RemoteUrl {
                    url: "http://example.com".into(),
                },
                None,
            )
            .unwrap();
        project.mark_ready_for_processing().unwrap();
        project_repo.create(project.clone()).await.unwrap();

        let use_case = StartMockPipelineUseCase::new(
            project_repo.clone(),
            job_scheduler.clone(),
            tx_gateway.clone(),
        );
        let request = StartMockPipelineRequest {
            project_id: project.id().clone(),
        };

        let response = use_case.execute(request).await.unwrap();

        assert_eq!(response.job.status, JobStatus::Running); // MockJobScheduler sets to Running

        // Verify transaction gateway received the save project and save job
        let projects_saved = tx_gateway.projects_saved.lock().await;
        assert_eq!(projects_saved.len(), 1);
        assert_eq!(
            projects_saved[0].status(),
            &domain::project::ProjectStatus::Processing
        );

        let jobs_saved = tx_gateway.jobs_saved.lock().await;
        assert_eq!(jobs_saved.len(), 1);
        assert_eq!(jobs_saved[0].status(), &JobStatus::Pending); // The usecase saves as pending!
    }

    #[tokio::test]
    async fn test_transaction_failure_does_not_enqueue_job() {
        let project_repo = InMemoryProjectRepository::new();
        let job_scheduler = Arc::new(MockJobScheduler::new());
        let tx_gateway = Arc::new(MockTransactionGateway::with_failure()); // Will fail

        let mut project = Project::new("Test".to_string());
        project
            .import_source(
                domain::media::MediaSource::RemoteUrl {
                    url: "http://example.com".into(),
                },
                None,
            )
            .unwrap();
        project.mark_ready_for_processing().unwrap();
        project_repo.create(project.clone()).await.unwrap();

        let use_case = StartMockPipelineUseCase::new(
            project_repo.clone(),
            job_scheduler.clone(),
            tx_gateway.clone(),
        );
        let request = StartMockPipelineRequest {
            project_id: project.id().clone(),
        };

        let response = use_case.execute(request).await;
        assert!(matches!(
            response,
            Err(ApplicationError::InvalidOperation { .. })
        ));

        // Verify job was NOT enqueued
        let scheduled_jobs = job_scheduler.jobs.lock().await;
        assert_eq!(scheduled_jobs.len(), 0);
    }

    #[tokio::test]
    async fn test_cannot_start_from_draft() {
        let project_repo = InMemoryProjectRepository::new();
        let job_scheduler = Arc::new(MockJobScheduler::new());
        let tx_gateway = Arc::new(MockTransactionGateway::new());

        let project = Project::new("Test".to_string());
        project_repo.create(project.clone()).await.unwrap();

        let use_case = StartMockPipelineUseCase::new(
            project_repo.clone(),
            job_scheduler.clone(),
            tx_gateway.clone(),
        );
        let request = StartMockPipelineRequest {
            project_id: project.id().clone(),
        };

        let response = use_case.execute(request).await;
        assert!(matches!(response, Err(ApplicationError::Domain { .. })));
    }

    #[tokio::test]
    async fn test_cannot_start_from_completed() {
        let project_repo = InMemoryProjectRepository::new();
        let job_scheduler = Arc::new(MockJobScheduler::new());
        let tx_gateway = Arc::new(MockTransactionGateway::new());

        let mut project = Project::new("Test".to_string());
        // Forcibly set to Completed is difficult without proper transitions, but we can try to do:
        project
            .import_source(
                domain::media::MediaSource::RemoteUrl {
                    url: "http://example.com".into(),
                },
                None,
            )
            .unwrap();
        project.mark_ready_for_processing().unwrap();
        project.mark_processing_started().unwrap();
        project.mark_completed().unwrap();
        project_repo.create(project.clone()).await.unwrap();

        let use_case = StartMockPipelineUseCase::new(
            project_repo.clone(),
            job_scheduler.clone(),
            tx_gateway.clone(),
        );
        let request = StartMockPipelineRequest {
            project_id: project.id().clone(),
        };

        let response = use_case.execute(request).await;
        assert!(matches!(response, Err(ApplicationError::Domain { .. })));
    }
}
