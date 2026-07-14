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

pub struct JobLifecycleCoordinator<
    R: ProjectRepository + Clone + 'static,
    E: AppEventPublisher + Clone + 'static,
> {
    app_event_publisher: E,
    handle_completed: HandleJobCompletedUseCase<R>,
    handle_cancelled: HandleJobCancelledUseCase<R>,
}

impl<R: ProjectRepository + Clone + 'static, E: AppEventPublisher + Clone + 'static>
    JobLifecycleCoordinator<R, E>
{
    pub fn new(project_repo: R, app_event_publisher: E) -> Self {
        Self {
            app_event_publisher,
            handle_completed: HandleJobCompletedUseCase::new(project_repo.clone()),
            handle_cancelled: HandleJobCancelledUseCase::new(project_repo),
        }
    }

    pub async fn handle(&self, event: JobLifecycleEvent) -> Result<(), ApplicationError> {
        let project_id_str = match event.project_id {
            Some(pid) => pid.to_string(),
            None => return Ok(()), // No-op if no project_id
        };

        match event.status {
            JobStatus::Completed | JobStatus::Failed => {
                let is_success = event.status == JobStatus::Completed;

                let result = self
                    .handle_completed
                    .execute(HandleJobCompletedRequest {
                        job_id: event.job_id.to_string(),
                        project_id: project_id_str.clone(),
                        is_success,
                    })
                    .await?;

                if is_success && result.transcript_ready {
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
                self.handle_cancelled
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
mod tests {
    use super::*;
    use async_trait::async_trait;
    use domain::job::{JobId, JobProgress, JobStatus};
    use domain::project::{Project, ProjectId};
    use ports::error::PortError;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct MockProjectRepo {
        project: Arc<Mutex<Option<Project>>>,
        fail_next_save: Arc<Mutex<bool>>,
    }

    impl MockProjectRepo {
        fn new(project: Option<Project>) -> Self {
            Self {
                project: Arc::new(Mutex::new(project)),
                fail_next_save: Arc::new(Mutex::new(false)),
            }
        }
    }

    #[async_trait]
    impl ProjectRepository for MockProjectRepo {
        async fn get(&self, _id: &ProjectId) -> Result<Option<Project>, PortError> {
            let guard = self.project.lock().unwrap();
            Ok(guard.clone())
        }

        async fn save(&self, project: &Project) -> Result<(), PortError> {
            if *self.fail_next_save.lock().unwrap() {
                *self.fail_next_save.lock().unwrap() = false;
                return Err(PortError::Unexpected {
                    message: "repo failure".into(),
                });
            }
            let mut guard = self.project.lock().unwrap();
            *guard = Some(project.clone());
            Ok(())
        }

        async fn create(&self, _project: Project) -> Result<Project, PortError> {
            unimplemented!()
        }

        async fn list(&self) -> Result<Vec<Project>, PortError> {
            unimplemented!()
        }

        async fn delete(&self, _id: &ProjectId) -> Result<(), PortError> {
            unimplemented!()
        }
    }

    #[derive(Clone, Default)]
    struct MockAppEventPublisher {
        events: Arc<Mutex<Vec<String>>>,
        fail_next: Arc<Mutex<bool>>,
    }

    impl MockAppEventPublisher {
        fn set_fail_next(&self, fail: bool) {
            *self.fail_next.lock().unwrap() = fail;
        }
    }

    #[async_trait]
    impl AppEventPublisher for MockAppEventPublisher {
        async fn publish_project_updated(&self, project_id: &str) -> Result<(), PortError> {
            if *self.fail_next.lock().unwrap() {
                *self.fail_next.lock().unwrap() = false;
                return Err(PortError::Unexpected {
                    message: "pub failure".into(),
                });
            }
            self.events
                .lock()
                .unwrap()
                .push(format!("project_updated:{}", project_id));
            Ok(())
        }

        async fn publish_transcript_ready(
            &self,
            project_id: &str,
            job_id: &str,
        ) -> Result<(), PortError> {
            if *self.fail_next.lock().unwrap() {
                *self.fail_next.lock().unwrap() = false;
                return Err(PortError::Unexpected {
                    message: "pub failure".into(),
                });
            }
            self.events
                .lock()
                .unwrap()
                .push(format!("transcript_ready:{}:{}", project_id, job_id));
            Ok(())
        }
    }

    fn create_processing_project() -> Project {
        let mut p = Project::new("Test".into());
        p.import_source(
            domain::media::MediaSource::ExternalLocalFile { path: "".into() },
            None,
        )
        .unwrap();
        p.mark_ready_for_processing().unwrap();
        p.start_processing(JobId::new()).unwrap();
        p
    }

    fn create_processing_youtube_project() -> Project {
        let mut p = Project::new("YT".into());
        p.import_source(
            domain::media::MediaSource::YoutubeUrl { url: "".into() },
            None,
        )
        .unwrap();
        p.mark_ready_for_processing().unwrap();
        p.start_processing(JobId::new()).unwrap();
        p
    }

    #[tokio::test]
    async fn test_queued_running_noop() {
        let repo = MockProjectRepo::new(Some(create_processing_project()));
        let publ = MockAppEventPublisher::default();
        let uc = JobLifecycleCoordinator::new(repo, publ.clone());

        for status in [JobStatus::Running, JobStatus::Pending] {
            let event = JobLifecycleEvent {
                job_id: JobId::new(),
                project_id: Some(ProjectId::new()),
                status,
                stage: None,
                progress: JobProgress::initializing(),
                error: None,
            };
            uc.handle(event).await.unwrap();
        }

        assert!(publ.events.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_no_project_id_noop() {
        let repo = MockProjectRepo::new(Some(create_processing_project()));
        let publ = MockAppEventPublisher::default();
        let uc = JobLifecycleCoordinator::new(repo, publ.clone());

        let event = JobLifecycleEvent {
            job_id: JobId::new(),
            project_id: None,
            status: JobStatus::Completed,
            stage: None,
            progress: JobProgress::initializing(),
            error: None,
        };
        uc.handle(event).await.unwrap();

        assert!(publ.events.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_project_not_found() {
        let repo = MockProjectRepo::new(None);
        let publ = MockAppEventPublisher::default();
        let uc = JobLifecycleCoordinator::new(repo, publ.clone());

        let event = JobLifecycleEvent {
            job_id: JobId::new(),
            project_id: Some(ProjectId::new()),
            status: JobStatus::Completed,
            stage: None,
            progress: JobProgress::initializing(),
            error: None,
        };

        let res = uc.handle(event).await;
        assert!(matches!(res, Err(ApplicationError::ProjectNotFound(_))));
    }

    #[tokio::test]
    async fn test_completed_local_media() {
        let p = create_processing_project();
        let pid_str = p.id().to_string();
        let repo = MockProjectRepo::new(Some(p.clone()));
        let publ = MockAppEventPublisher::default();
        let uc = JobLifecycleCoordinator::new(repo.clone(), publ.clone());

        let event = JobLifecycleEvent {
            job_id: JobId::new(),
            project_id: Some(p.id().clone()),
            status: JobStatus::Completed,
            stage: None,
            progress: JobProgress::initializing(),
            error: None,
        };
        uc.handle(event).await.unwrap();

        let events = publ.events.lock().unwrap().clone();
        assert_eq!(events, vec![format!("project_updated:{}", pid_str)]);
    }

    #[tokio::test]
    async fn test_completed_with_transcript_ready() {
        let mut p = create_processing_project();
        let pid_str = p.id().to_string();
        p.set_transcript(domain::transcript::Transcript {
            language: "en".into(),
            segments: vec![],
        });
        let repo = MockProjectRepo::new(Some(p.clone()));
        let publ = MockAppEventPublisher::default();
        let uc = JobLifecycleCoordinator::new(repo.clone(), publ.clone());

        let job_id = JobId::new();
        let event = JobLifecycleEvent {
            job_id: job_id.clone(),
            project_id: Some(p.id().clone()),
            status: JobStatus::Completed,
            stage: None,
            progress: JobProgress::initializing(),
            error: None,
        };
        uc.handle(event).await.unwrap();

        let events = publ.events.lock().unwrap().clone();
        assert_eq!(
            events,
            vec![
                format!("transcript_ready:{}:{}", pid_str, job_id),
                format!("project_updated:{}", pid_str)
            ]
        );
    }

    #[tokio::test]
    async fn test_completed_youtube_subtitles_fail() {
        let p = create_processing_youtube_project();
        let pid_str = p.id().to_string();
        let repo = MockProjectRepo::new(Some(p.clone()));
        let publ = MockAppEventPublisher::default();
        let uc = JobLifecycleCoordinator::new(repo.clone(), publ.clone());

        let event = JobLifecycleEvent {
            job_id: JobId::new(),
            project_id: Some(p.id().clone()),
            status: JobStatus::Completed,
            stage: None,
            progress: JobProgress::initializing(),
            error: None,
        };
        uc.handle(event).await.unwrap();

        let events = publ.events.lock().unwrap().clone();
        assert_eq!(events, vec![format!("project_updated:{}", pid_str)]);
    }

    #[tokio::test]
    async fn test_failed() {
        let p = create_processing_project();
        let pid_str = p.id().to_string();
        let repo = MockProjectRepo::new(Some(p.clone()));
        let publ = MockAppEventPublisher::default();
        let uc = JobLifecycleCoordinator::new(repo.clone(), publ.clone());

        let event = JobLifecycleEvent {
            job_id: JobId::new(),
            project_id: Some(p.id().clone()),
            status: JobStatus::Failed,
            stage: None,
            progress: JobProgress::initializing(),
            error: None,
        };
        uc.handle(event).await.unwrap();

        let events = publ.events.lock().unwrap().clone();
        assert_eq!(events, vec![format!("project_updated:{}", pid_str)]);
    }

    #[tokio::test]
    async fn test_failed_with_existing_transcript_no_ready_event() {
        let mut p = create_processing_project();
        let pid_str = p.id().to_string();
        p.set_transcript(domain::transcript::Transcript {
            language: "en".into(),
            segments: vec![],
        });
        let repo = MockProjectRepo::new(Some(p.clone()));
        let publ = MockAppEventPublisher::default();
        let uc = JobLifecycleCoordinator::new(repo.clone(), publ.clone());

        let event = JobLifecycleEvent {
            job_id: JobId::new(),
            project_id: Some(p.id().clone()),
            status: JobStatus::Failed,
            stage: None,
            progress: JobProgress::initializing(),
            error: None,
        };
        uc.handle(event).await.unwrap();

        let events = publ.events.lock().unwrap().clone();
        // Should NOT emit transcript_ready since job failed
        assert_eq!(events, vec![format!("project_updated:{}", pid_str)]);
    }

    #[tokio::test]
    async fn test_cancelled() {
        let p = create_processing_project();
        let pid_str = p.id().to_string();
        let repo = MockProjectRepo::new(Some(p.clone()));
        let publ = MockAppEventPublisher::default();
        let uc = JobLifecycleCoordinator::new(repo.clone(), publ.clone());

        let event = JobLifecycleEvent {
            job_id: JobId::new(),
            project_id: Some(p.id().clone()),
            status: JobStatus::Cancelled,
            stage: None,
            progress: JobProgress::initializing(),
            error: None,
        };
        uc.handle(event).await.unwrap();

        let events = publ.events.lock().unwrap().clone();
        assert_eq!(events, vec![format!("project_updated:{}", pid_str)]);
    }

    #[tokio::test]
    async fn test_publisher_failure() {
        let p = create_processing_project();
        let repo = MockProjectRepo::new(Some(p.clone()));
        let publ = MockAppEventPublisher::default();
        publ.set_fail_next(true);
        let uc = JobLifecycleCoordinator::new(repo.clone(), publ.clone());

        let event = JobLifecycleEvent {
            job_id: JobId::new(),
            project_id: Some(p.id().clone()),
            status: JobStatus::Completed,
            stage: None,
            progress: JobProgress::initializing(),
            error: None,
        };
        let res = uc.handle(event).await;
        assert!(res.is_err());
    }
}
