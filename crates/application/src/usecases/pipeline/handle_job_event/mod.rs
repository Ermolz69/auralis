use crate::error::ApplicationError;
use crate::usecases::project::handle_job_cancelled::{
    HandleJobCancelledRequest, HandleJobCancelledUseCase,
};
use crate::usecases::project::handle_job_completed::{
    HandleJobCompletedRequest, HandleJobCompletedUseCase,
};
use domain::job::JobStatus;
use ports::events::AppEventPublisher;
use ports::job_scheduler::JobLifecycleEvent;
use ports::repository::ProjectRepository;

pub struct HandleJobEventUseCase<
    R: ProjectRepository + Clone + 'static,
    E: AppEventPublisher + Clone + 'static,
> {
    project_repo: R,
    app_event_publisher: E,
}

impl<R: ProjectRepository + Clone + 'static, E: AppEventPublisher + Clone + 'static>
    HandleJobEventUseCase<R, E>
{
    pub fn new(project_repo: R, app_event_publisher: E) -> Self {
        Self {
            project_repo,
            app_event_publisher,
        }
    }

    pub async fn execute(&self, event: JobLifecycleEvent) -> Result<(), ApplicationError> {
        let project_id_str = match event.project_id {
            Some(pid) => pid.to_string(),
            None => return Ok(()), // No-op if no project_id
        };

        match event.status {
            JobStatus::Completed | JobStatus::Failed => {
                let is_success = event.status == JobStatus::Completed;

                let use_case = HandleJobCompletedUseCase::new(self.project_repo.clone());

                let result = use_case
                    .execute(HandleJobCompletedRequest {
                        job_id: event.job_id.to_string(),
                        project_id: project_id_str.clone(),
                        is_success,
                    })
                    .await?;

                if result.transcript_ready {
                    let job_id_str = event.job_id.to_string();
                    self.app_event_publisher
                        .publish_transcript_ready(&project_id_str, &job_id_str)
                        .await?;
                }
                self.app_event_publisher
                    .publish_project_updated(&project_id_str)
                    .await?;
            }
            JobStatus::Cancelled => {
                let use_case = HandleJobCancelledUseCase::new(self.project_repo.clone());
                use_case
                    .execute(HandleJobCancelledRequest {
                        job_id: event.job_id.to_string(),
                        project_id: project_id_str.clone(),
                    })
                    .await?;

                self.app_event_publisher
                    .publish_project_updated(&project_id_str)
                    .await?;
            }
            JobStatus::Running | JobStatus::Pending => {
                // no-op
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests;
