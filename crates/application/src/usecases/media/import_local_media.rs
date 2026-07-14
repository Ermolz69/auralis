use std::path::PathBuf;
use std::sync::Arc;

use domain::project::{Project, ProjectId};
use ports::job_scheduler::{JobSchedulerPort, ScheduledJob};
use ports::media::MediaProbePort;
use ports::repository::ProjectRepository;
use ports::source::SubtitleSourcePort;
use ports::storage::ArtifactStore;
use ports::transaction::StorageUnitOfWork;

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
    V: SubtitleSourcePort + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> {
    project_repo: R,
    media_probe: P,
    job_scheduler: Arc<dyn JobSchedulerPort>,
    storage_uow: Arc<dyn StorageUnitOfWork>,
    subtitle_source: V,
    artifact_store: S,
    artifact_index: Arc<dyn ports::artifact_index::ArtifactIndex>,
    target_dir_base: std::path::PathBuf,
}

impl<
    R: ProjectRepository + Clone + 'static,
    P: MediaProbePort + Clone + 'static,
    V: SubtitleSourcePort + Clone + 'static,
    S: ArtifactStore + Clone + 'static,
> ImportLocalMediaUseCase<R, P, V, S>
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        project_repo: R,
        media_probe: P,
        job_scheduler: Arc<dyn JobSchedulerPort>,
        storage_uow: Arc<dyn StorageUnitOfWork>,
        subtitle_source: V,
        artifact_store: S,
        artifact_index: Arc<dyn ports::artifact_index::ArtifactIndex>,
        target_dir_base: std::path::PathBuf,
    ) -> Self {
        Self {
            project_repo,
            media_probe,
            job_scheduler,
            storage_uow,
            subtitle_source,
            artifact_store,
            artifact_index,
            target_dir_base,
        }
    }

    pub async fn execute(
        &self,
        request: ImportLocalMediaRequest,
    ) -> Result<ImportLocalMediaResponse, ApplicationError> {
        let probe_use_case = ProbeLocalMediaUseCase::new(self.media_probe.clone());
        let probe_req = ProbeLocalMediaRequest {
            path: request.path.clone(),
        };

        // This will probe the source
        let probe_res = probe_use_case.execute(probe_req).await?;

        let original_filename = request
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string());

        // 1. Stage the file
        let staged_artifact = self
            .artifact_store
            .import_external_file(
                &request.project_id,
                domain::media::ArtifactKind::SourceVideo,
                &request.path,
                original_filename.as_deref(),
            )
            .await?;

        // 2. Load project
        let mut project = self
            .project_repo
            .get(&request.project_id)
            .await?
            .ok_or_else(|| ApplicationError::ProjectNotFound(request.project_id.clone()))?;

        // 3. Attach ManagedLocalFile
        let source = domain::media::MediaSource::ManagedLocalFile {
            artifact_id: staged_artifact.artifact.id.clone(),
            original_filename: original_filename.unwrap_or_else(|| "video.mp4".to_string()),
        };
        project.import_source(source, Some(probe_res.metadata.clone()))?;

        // 4. Commit managed source import
        let commit_cmd = ports::transaction::CommitManagedSourceImport {
            project: project.clone(),
            artifact: staged_artifact.artifact.clone(),
            staging_key: staged_artifact.staging_key.clone(),
            final_key: staged_artifact.final_key.clone(),
        };
        self.storage_uow
            .commit_managed_source_import(commit_cmd)
            .await?;

        // 5. Fast-path finalization
        let finalizer = crate::services::artifact_finalizer::ArtifactFinalizer::new(
            self.artifact_index.clone(),
            self.artifact_store.clone(),
        );

        finalizer
            .finalize(
                &staged_artifact.artifact.id,
                &staged_artifact.staging_key,
                &staged_artifact.final_key,
            )
            .await?;

        // 6. Transition project to ReadyForProcessing
        let mut project = self.project_repo.get(&request.project_id).await?.unwrap();
        project.mark_ready_for_processing()?;
        self.project_repo.save(&project).await?;

        let pipeline_use_case = StartMockPipelineUseCase::new(
            self.project_repo.clone(),
            self.job_scheduler.clone(),
            self.storage_uow.clone(),
            self.subtitle_source.clone(),
            self.artifact_store.clone(),
            self.target_dir_base.clone(),
        );

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
    use async_trait::async_trait;
    use domain::job::JobStatus;
    use domain::media::SubtitleTrack;
    use domain::project::ProjectStatus;
    use ports::error::PortError;
    use std::fs::File;
    use tempfile::tempdir;

    #[derive(Clone)]
    struct MockSubtitleSource;

    #[async_trait]
    impl SubtitleSourcePort for MockSubtitleSource {
        async fn list_subtitles(
            &self,
            _source: &domain::media::MediaSource,
        ) -> Result<Vec<SubtitleTrack>, PortError> {
            Ok(vec![])
        }

        async fn download_subtitle(
            &self,
            _source: &domain::media::MediaSource,
            _track: &SubtitleTrack,
            _target_path: &std::path::Path,
        ) -> Result<domain::media::Artifact, PortError> {
            Err(PortError::Unsupported {
                message: "Not implemented".into(),
            })
        }
    }

    use crate::test_utils::{MockArtifactIndex, MockArtifactStore};

    #[tokio::test]
    async fn imports_local_media_and_starts_pipeline() {
        let repo = InMemoryProjectRepository::new();
        let probe = MockMediaProbeAdapter::new();
        let job_scheduler = Arc::new(MockJobScheduler::new());
        let tx_gateway = std::sync::Arc::new(crate::test_utils::MockStorageUnitOfWork::new());
        let use_case = ImportLocalMediaUseCase::new(
            repo.clone(),
            probe,
            job_scheduler.clone(),
            tx_gateway.clone(),
            MockSubtitleSource,
            Arc::new(MockArtifactStore),
            Arc::new(MockArtifactIndex::new()),
            std::path::PathBuf::from("/tmp"),
        );

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

        let projects_saved = tx_gateway.projects_saved.lock().await;
        let saved_project = projects_saved
            .iter()
            .find(|p| p.id() == &project_id)
            .unwrap();
        assert_eq!(*saved_project.status(), ProjectStatus::Processing);
    }

    #[tokio::test]
    async fn returns_error_when_project_missing() {
        let repo = InMemoryProjectRepository::new();
        let probe = MockMediaProbeAdapter::new();
        let job_scheduler = Arc::new(MockJobScheduler::new());
        let use_case = ImportLocalMediaUseCase::new(
            repo.clone(),
            probe,
            job_scheduler.clone(),
            std::sync::Arc::new(crate::test_utils::MockStorageUnitOfWork::new()),
            MockSubtitleSource,
            Arc::new(MockArtifactStore),
            Arc::new(MockArtifactIndex::new()),
            std::path::PathBuf::from("/tmp"),
        );

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
        let use_case = ImportLocalMediaUseCase::new(
            repo.clone(),
            probe,
            job_scheduler.clone(),
            std::sync::Arc::new(crate::test_utils::MockStorageUnitOfWork::new()),
            MockSubtitleSource,
            Arc::new(MockArtifactStore),
            Arc::new(MockArtifactIndex::new()),
            std::path::PathBuf::from("/tmp"),
        );

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
        let use_case = ImportLocalMediaUseCase::new(
            repo.clone(),
            probe,
            job_scheduler.clone(),
            std::sync::Arc::new(crate::test_utils::MockStorageUnitOfWork::new()),
            MockSubtitleSource,
            Arc::new(MockArtifactStore),
            Arc::new(MockArtifactIndex::new()),
            std::path::PathBuf::from("/tmp"),
        );

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
        let use_case = ImportLocalMediaUseCase::new(
            repo.clone(),
            probe,
            job_scheduler.clone(),
            std::sync::Arc::new(crate::test_utils::MockStorageUnitOfWork::new()),
            MockSubtitleSource,
            Arc::new(MockArtifactStore),
            Arc::new(MockArtifactIndex::new()),
            std::path::PathBuf::from("/tmp"),
        );

        let mut project = Project::new("Test Probe".to_string());
        let project_id = project.id().clone();
        // Force the project into a status that can't be marked ready for processing.
        // Wait, Draft -> SourceImported (via probe) -> ReadyForProcessing is allowed.
        // If we force the project into Completed, then probe will try to import source,
        // which might fail if it's Completed.
        project
            .import_source(
                domain::media::MediaSource::ExternalLocalFile {
                    path: "".to_string(),
                },
                None,
            )
            .unwrap();
        project.mark_ready_for_processing().unwrap();
        let test_job_id = domain::job::JobId::new();
        project.start_processing(test_job_id.clone()).unwrap();
        project
            .apply_terminal_transition(&test_job_id, domain::job::TerminalOutcome::Completed)
            .unwrap();

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
