use super::*;
use crate::test_utils::{MockArtifactStore, MockStorageUnitOfWork};
use adapters_storage::{local::LocalTempWorkspace, memory::InMemoryProjectRepository};
use async_trait::async_trait;
use domain::job::JobStatus;
use ports::job_scheduler::ScheduledJob;
use std::sync::Arc;
use tokio::sync::Mutex;

struct MatrixScheduler {
    update_calls: Arc<Mutex<usize>>,
    fail_update_call: Option<usize>,
    fail_job_error: Option<PortError>,
    fail_records: Arc<Mutex<Vec<(String, String)>>>,
}

impl MatrixScheduler {
    fn new(fail_update_call: Option<usize>, fail_job_error: Option<PortError>) -> Self {
        Self {
            update_calls: Arc::new(Mutex::new(0)),
            fail_update_call,
            fail_job_error,
            fail_records: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl JobSchedulerPort for MatrixScheduler {
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
        let mut calls = self.update_calls.lock().await;
        *calls += 1;
        if self.fail_update_call == Some(*calls) {
            return Err(PortError::Storage {
                operation: "update_job_stage",
                message: "adapter path must not persist".into(),
            });
        }
        Ok(scheduled(job_id, JobStatus::Running))
    }

    async fn complete_job(&self, job_id: &JobId) -> Result<ScheduledJob, PortError> {
        Ok(scheduled(job_id, JobStatus::Completed))
    }

    async fn fail_job(
        &self,
        job_id: &JobId,
        code: String,
        message: String,
        _retryable: bool,
    ) -> Result<ScheduledJob, PortError> {
        self.fail_records.lock().await.push((code, message));
        if let Some(err) = &self.fail_job_error {
            return Err(clone_port_error(err));
        }
        Ok(scheduled(job_id, JobStatus::Failed))
    }
}

#[derive(Clone)]
struct EmptySubtitleSource;

#[async_trait]
impl SubtitleSourcePort for EmptySubtitleSource {
    async fn list_subtitles(
        &self,
        _source: &domain::media::MediaSource,
    ) -> Result<Vec<domain::media::SubtitleTrack>, PortError> {
        Ok(vec![])
    }

    async fn download_subtitle(
        &self,
        _request: ports::source::DownloadSubtitleRequest,
    ) -> Result<domain::media::Artifact, PortError> {
        unreachable!()
    }
}

#[tokio::test]
async fn stage_update_failures_terminalize_through_one_helper() {
    for fail_call in 1..=4 {
        let scheduler = Arc::new(MatrixScheduler::new(Some(fail_call), None));
        let outcome = run_with_scheduler(scheduler.clone()).await;

        assert_eq!(outcome, RuntimeTaskOutcome::ApplicationFailed);
        let records = scheduler.fail_records.lock().await;
        assert_eq!(
            records.as_slice(),
            &[(
                "STAGE_UPDATE_FAILED".into(),
                "Pipeline progress could not be persisted.".into()
            )]
        );
    }
}

#[tokio::test]
async fn subtitle_failure_is_terminalized_with_sanitized_error() {
    let scheduler = Arc::new(MatrixScheduler::new(None, None));
    let outcome = run_with_scheduler(scheduler.clone()).await;

    assert_eq!(outcome, RuntimeTaskOutcome::ApplicationFailed);
    let records = scheduler.fail_records.lock().await;
    assert_eq!(
        records.as_slice(),
        &[(
            "SUBTITLE_IMPORT_FAILED".into(),
            "Subtitle import failed.".into()
        )]
    );
}

#[tokio::test]
async fn fail_job_persistence_failure_requires_recovery() {
    let scheduler = Arc::new(MatrixScheduler::new(
        Some(1),
        Some(PortError::Storage {
            operation: "fail_job",
            message: "database unavailable".into(),
        }),
    ));

    let outcome = run_with_scheduler(scheduler).await;
    assert_eq!(outcome, RuntimeTaskOutcome::RecoveryRequired);
}

#[tokio::test]
async fn deleted_job_is_terminal_noop() {
    let scheduler = Arc::new(MatrixScheduler::new(
        Some(1),
        Some(PortError::NotFound {
            resource: "Job".into(),
        }),
    ));

    let outcome = run_with_scheduler(scheduler).await;
    assert_eq!(outcome, RuntimeTaskOutcome::DeletedNoOp);
}

#[tokio::test]
async fn cancellation_wins_during_await() {
    let (handle, token) = ports::cancellation::CancelHandle::new();
    let pending = async {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    };
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        handle.cancel();
    });

    let outcome = super::super::runner_terminalization::await_or_cancel(&token, pending).await;
    assert_eq!(outcome, Err(RuntimeTaskOutcome::Cancelled));
}

async fn run_with_scheduler(scheduler: Arc<MatrixScheduler>) -> RuntimeTaskOutcome {
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
        EmptySubtitleSource,
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

    runner
        .run(JobId::new(), project_id, token, &mut guard)
        .await
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

fn clone_port_error(err: &PortError) -> PortError {
    match err {
        PortError::NotFound { resource } => PortError::NotFound {
            resource: resource.clone(),
        },
        PortError::Storage { operation, message } => PortError::Storage {
            operation,
            message: message.clone(),
        },
        _ => PortError::Unexpected {
            message: "test error".into(),
        },
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
