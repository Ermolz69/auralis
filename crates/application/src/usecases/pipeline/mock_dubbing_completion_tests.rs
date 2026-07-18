use super::*;
use crate::test_utils::{MockArtifactStore, MockStorageUnitOfWork};
use adapters_storage::{local::LocalTempWorkspace, memory::InMemoryProjectRepository};
use async_trait::async_trait;
use domain::job::JobStatus;
use ports::job_scheduler::ScheduledJob;
use std::sync::Arc;
use tokio::sync::Mutex;

struct CompletionScheduler {
    fail_records: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl JobSchedulerPort for CompletionScheduler {
    async fn start_dubbing_job(
        &self,
        _request: ports::job_scheduler::StartDubbingJobRequest,
    ) -> Result<ScheduledJob, PortError> {
        unreachable!()
    }

    async fn enqueue_existing_job(&self, _job_id: &JobId) -> Result<ScheduledJob, PortError> {
        unreachable!()
    }

    async fn cancel_job(&self, job_id: &JobId) -> Result<ScheduledJob, PortError> {
        Ok(scheduled(job_id, JobStatus::Cancelled))
    }

    async fn get_job(&self, job_id: &JobId) -> Result<Option<ScheduledJob>, PortError> {
        Ok(Some(scheduled(job_id, JobStatus::Running)))
    }

    async fn list_jobs(&self) -> Result<Vec<ScheduledJob>, PortError> {
        Ok(vec![])
    }

    async fn update_job_stage(
        &self,
        job_id: &JobId,
        _stage: DubbingPipelineStage,
        _progress: JobProgress,
    ) -> Result<ScheduledJob, PortError> {
        Ok(scheduled(job_id, JobStatus::Running))
    }

    async fn complete_job(&self, _job_id: &JobId) -> Result<ScheduledJob, PortError> {
        Err(PortError::Storage {
            operation: "complete_job",
            message: "commit failed".into(),
        })
    }

    async fn fail_job(
        &self,
        job_id: &JobId,
        _code: String,
        message: String,
        _retryable: bool,
    ) -> Result<ScheduledJob, PortError> {
        self.fail_records.lock().await.push(message);
        Ok(scheduled(job_id, JobStatus::Failed))
    }
}

#[derive(Clone)]
struct SuccessfulSubtitleSource;

#[async_trait]
impl SubtitleSourcePort for SuccessfulSubtitleSource {
    async fn list_subtitles(
        &self,
        _source: &domain::media::MediaSource,
    ) -> Result<Vec<domain::media::SubtitleTrack>, PortError> {
        Ok(vec![domain::media::SubtitleTrack {
            id: "sub".into(),
            language: "en".into(),
            label: None,
            format: Some("vtt".into()),
            is_auto_generated: false,
        }])
    }

    async fn download_subtitle(
        &self,
        request: ports::source::DownloadSubtitleRequest,
    ) -> Result<domain::media::Artifact, PortError> {
        let path = request.target_directory.join("sub.vtt");
        tokio::fs::write(&path, "WEBVTT\n\n00:00:00.000 --> 00:00:01.000\nHi")
            .await
            .unwrap();
        Ok(domain::media::Artifact {
            id: domain::media::ArtifactId::new(),
            kind: domain::media::ArtifactKind::OriginalSubtitle,
            location: domain::media::ArtifactLocation::LocalPath(path.to_string_lossy().into()),
            size_bytes: Some(10),
            state: domain::media::ArtifactState::PendingFinalize,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            ready_at: None,
        })
    }
}

#[tokio::test]
async fn completion_failure_requires_recovery_without_fail_job_overwrite() {
    let fail_records = Arc::new(Mutex::new(Vec::new()));
    let scheduler = Arc::new(CompletionScheduler {
        fail_records: fail_records.clone(),
    });
    let project_repo = InMemoryProjectRepository::new(Arc::new(std::sync::Mutex::new(
        adapters_storage::memory::InMemoryDatabase::new(),
    )));
    let mut project = domain::project::Project::new("Test".into());
    project
        .import_source(
            domain::media::MediaSource::RemoteUrl {
                url: "https://example.invalid/video".into(),
            },
            None,
        )
        .unwrap();
    project.mark_ready_for_processing().unwrap();
    let project_id = project.id().clone();
    project_repo.create(project).await.unwrap();

    let runner = MockDubbingPipelineRunner::new(
        scheduler,
        project_repo,
        SuccessfulSubtitleSource,
        MockStorageUnitOfWork::new(),
        MockArtifactStore,
        Arc::new(LocalTempWorkspace::new(std::env::temp_dir())),
        Arc::new(NoopRuntime),
    );
    let (_handle, token) = ports::cancellation::CancelHandle::new();
    let span = tracing::info_span!("test_job_execution");
    let mut guard = crate::observability::execution_summary::ExecutionSummaryGuard::new(
        span,
        crate::observability::execution_summary::OperationSummary::JobExecution {
            project_id: project_id.to_string(),
            job_id: "test".into(),
            action: "job_execution",
            status: "started".into(),
        },
    );

    let outcome = runner
        .run(JobId::new(), project_id, token, &mut guard)
        .await;
    assert_eq!(outcome, RuntimeTaskOutcome::RecoveryRequired);
    assert!(fail_records.lock().await.is_empty());
}

fn scheduled(job_id: &JobId, status: JobStatus) -> ScheduledJob {
    ScheduledJob {
        id: job_id.clone(),
        revision: 1,
        project_id: None,
        title: "Job".into(),
        status,
        stage: None,
        progress: JobProgress::initializing(),
        error: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

struct NoopRuntime;

#[async_trait]
impl ports::job_runtime_control::JobRuntimeControlPort for NoopRuntime {
    async fn reserve(&self, _job_id: JobId, _project_id: ProjectId) -> Result<(), PortError> {
        Ok(())
    }

    async fn attach_task(
        &self,
        _job_id: JobId,
        _task: ports::job_runtime_control::RuntimeTask,
    ) -> Result<(), ports::job_runtime_control::AttachTaskError> {
        Ok(())
    }

    fn finish_now(&self, _job_id: &JobId) {}

    async fn rollback_runtime_start(
        &self,
        _job_id: &JobId,
    ) -> Result<ports::job_runtime_control::RuntimeCleanupOutcome, PortError> {
        Ok(ports::job_runtime_control::RuntimeCleanupOutcome::Missing)
    }

    async fn cancel_and_evict_jobs(
        &self,
        _job_ids: &[JobId],
    ) -> Result<ports::job_runtime_control::RuntimeCleanupReport, PortError> {
        Ok(ports::job_runtime_control::RuntimeCleanupReport {
            jobs: std::collections::HashMap::new(),
        })
    }
}
