use std::path::PathBuf;
use std::sync::Arc;

use domain::project::{Project, ProjectId};
use ports::job_scheduler::{JobSchedulerPort, ScheduledJob};
use ports::media::MediaProbePort;
use ports::repository::ProjectRepository;

use crate::error::ApplicationError;
use crate::usecases::media::probe_local::{ProbeLocalMediaRequest, ProbeLocalMediaUseCase};
use crate::usecases::pipeline::start_mock::{StartMockPipelineRequest, StartMockPipelineUseCase};

#[derive(Debug)]
pub struct ImportLocalMediaRequest {
    pub project_id: ProjectId,
    pub path: PathBuf,
}

#[derive(Debug)]
pub struct ImportLocalMediaResponse {
    pub project: Project,
    pub job: ScheduledJob,
}

pub struct ImportLocalMediaUseCase<
    R: ProjectRepository + Clone + 'static,
    P: MediaProbePort + Clone + 'static,
> {
    project_repo: R,
    media_probe: P,
    job_scheduler: Arc<dyn JobSchedulerPort>,
}

impl<R: ProjectRepository + Clone + 'static, P: MediaProbePort + Clone + 'static>
    ImportLocalMediaUseCase<R, P>
{
    pub fn new(project_repo: R, media_probe: P, job_scheduler: Arc<dyn JobSchedulerPort>) -> Self {
        Self {
            project_repo,
            media_probe,
            job_scheduler,
        }
    }

    pub async fn execute(
        &self,
        request: ImportLocalMediaRequest,
    ) -> Result<ImportLocalMediaResponse, ApplicationError> {
        let probe_use_case =
            ProbeLocalMediaUseCase::new(self.project_repo.clone(), self.media_probe.clone());
        let probe_req = ProbeLocalMediaRequest {
            project_id: Some(request.project_id.clone()),
            path: request.path,
        };

        // This will probe and import the source, saving it.
        let probe_res = probe_use_case.execute(probe_req).await?;

        let mut project = probe_res
            .project
            .ok_or_else(|| ApplicationError::InvalidOperation {
                message: "Probe local media did not return project".to_string(),
            })?;

        project.mark_ready_for_processing()?;
        self.project_repo.save(&project).await?;

        let pipeline_use_case =
            StartMockPipelineUseCase::new(self.project_repo.clone(), self.job_scheduler.clone());

        let pipeline_req = StartMockPipelineRequest {
            project_id: request.project_id,
        };

        let pipeline_res = pipeline_use_case.execute(pipeline_req).await?;

        Ok(ImportLocalMediaResponse {
            project: pipeline_res.project,
            job: pipeline_res.job,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockJobScheduler;
    use adapters_ffmpeg::mock::MockMediaProbeAdapter;
    use adapters_storage::memory::InMemoryProjectRepository;
    use domain::job::JobStatus;
    use domain::project::ProjectStatus;
    use std::fs::File;
    use tempfile::tempdir;

    #[tokio::test]
    async fn imports_local_media_and_starts_pipeline() {
        let repo = InMemoryProjectRepository::new();
        let probe = MockMediaProbeAdapter::new();
        let job_scheduler = Arc::new(MockJobScheduler::new());
        let use_case = ImportLocalMediaUseCase::new(repo.clone(), probe, job_scheduler);

        let project = Project::new("Test Probe".to_string());
        let project_id = project.id().clone();
        repo.create(project.clone()).await.unwrap();

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("video.mp4");
        File::create(&file_path).unwrap();

        let request = ImportLocalMediaRequest {
            project_id: project_id.clone(),
            path: file_path,
        };

        let response = use_case.execute(request).await.unwrap();

        assert_eq!(*response.project.status(), ProjectStatus::Processing);
        assert!(
            response.job.status == JobStatus::Pending || response.job.status == JobStatus::Running
        );

        let saved = repo.get(&project_id).await.unwrap().unwrap();
        assert_eq!(*saved.status(), ProjectStatus::Processing);
    }

    #[tokio::test]
    async fn returns_error_when_project_missing() {
        let repo = InMemoryProjectRepository::new();
        let probe = MockMediaProbeAdapter::new();
        let job_scheduler = Arc::new(MockJobScheduler::new());
        let use_case = ImportLocalMediaUseCase::new(repo.clone(), probe, job_scheduler);

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("video.mp4");
        File::create(&file_path).unwrap();

        let request = ImportLocalMediaRequest {
            project_id: ProjectId::new(),
            path: file_path,
        };

        let err = use_case.execute(request).await.unwrap_err();
        assert!(matches!(err, ApplicationError::ProjectNotFound(_)));
    }

    #[tokio::test]
    async fn returns_error_when_file_missing() {
        let repo = InMemoryProjectRepository::new();
        let probe = MockMediaProbeAdapter::new();
        let job_scheduler = Arc::new(MockJobScheduler::new());
        let use_case = ImportLocalMediaUseCase::new(repo.clone(), probe, job_scheduler);

        let project = Project::new("Test Probe".to_string());
        let project_id = project.id().clone();
        repo.create(project.clone()).await.unwrap();

        let request = ImportLocalMediaRequest {
            project_id: project_id.clone(),
            path: PathBuf::from("/non/existent/path.mp4"),
        };

        let err = use_case.execute(request).await.unwrap_err();
        assert!(matches!(err, ApplicationError::InvalidOperation { .. }));
    }

    // A mock probe that always fails to verify that the job is not started
    #[derive(Clone, Default)]
    struct FailingProbeAdapter;

    #[async_trait::async_trait]
    impl MediaProbePort for FailingProbeAdapter {
        async fn probe_local_file(
            &self,
            _path: &std::path::Path,
        ) -> Result<domain::media::MediaMetadata, ports::error::PortError> {
            Err(ports::error::PortError::Unexpected {
                message: "Probe failed".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn does_not_start_job_when_probe_fails() {
        let repo = InMemoryProjectRepository::new();
        let probe = FailingProbeAdapter;
        let job_scheduler = Arc::new(MockJobScheduler::new());
        let use_case = ImportLocalMediaUseCase::new(repo.clone(), probe, job_scheduler.clone());

        let project = Project::new("Test Probe".to_string());
        let project_id = project.id().clone();
        repo.create(project.clone()).await.unwrap();

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("video.mp4");
        File::create(&file_path).unwrap();

        let request = ImportLocalMediaRequest {
            project_id: project_id.clone(),
            path: file_path,
        };

        let err = use_case.execute(request).await.unwrap_err();
        assert!(matches!(err, ApplicationError::Port(_)));

        let saved = repo.get(&project_id).await.unwrap().unwrap();
        assert_eq!(*saved.status(), ProjectStatus::Draft);
    }

    #[tokio::test]
    async fn fails_on_invalid_project_transition() {
        let repo = InMemoryProjectRepository::new();
        let probe = MockMediaProbeAdapter::new();
        let job_scheduler = Arc::new(MockJobScheduler::new());
        let use_case = ImportLocalMediaUseCase::new(repo.clone(), probe, job_scheduler);

        let mut project = Project::new("Test Probe".to_string());
        let project_id = project.id().clone();
        // Force the project into a status that can't be marked ready for processing.
        // Wait, Draft -> SourceImported (via probe) -> ReadyForProcessing is allowed.
        // If we force the project into Completed, then probe will try to import source,
        // which might fail if it's Completed.
        project
            .import_source(
                domain::media::MediaSource::LocalFile {
                    path: "".to_string(),
                },
                None,
            )
            .unwrap();
        project.mark_ready_for_processing().unwrap();
        project.mark_processing_started().unwrap();
        project.mark_completed().unwrap();

        repo.create(project.clone()).await.unwrap();

        let dir = tempdir().unwrap();
        let file_path = dir.path().join("video.mp4");
        File::create(&file_path).unwrap();

        let request = ImportLocalMediaRequest {
            project_id: project_id.clone(),
            path: file_path,
        };

        // probe_local_file -> import_source -> fails because ProjectStatus is Completed!
        let err = use_case.execute(request).await.unwrap_err();
        assert!(matches!(err, ApplicationError::Domain(_)));

        let saved = repo.get(&project_id).await.unwrap().unwrap();
        assert_eq!(*saved.status(), ProjectStatus::Completed);
    }
}
