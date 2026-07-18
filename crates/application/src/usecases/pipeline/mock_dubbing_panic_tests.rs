use super::*;
use crate::test_utils::{MockArtifactStore, MockStorageUnitOfWork};
use adapters_storage::{local::LocalTempWorkspace, memory::InMemoryProjectRepository};
use async_trait::async_trait;
use ports::job_scheduler::ScheduledJob;
use std::sync::Arc;
use tokio::sync::Mutex;

struct PanicScheduler {
    fail_records: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl JobSchedulerPort for PanicScheduler {
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
        Ok(scheduled(job_id))
    }
    async fn get_job(&self, job_id: &JobId) -> Result<Option<ScheduledJob>, PortError> {
        Ok(Some(scheduled(job_id)))
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
        Ok(scheduled(job_id))
    }
    async fn complete_job(&self, job_id: &JobId) -> Result<ScheduledJob, PortError> {
        Ok(scheduled(job_id))
    }
    async fn fail_job(
        &self,
        job_id: &JobId,
        code: String,
        _message: String,
        _retryable: bool,
    ) -> Result<ScheduledJob, PortError> {
        self.fail_records.lock().await.push(code);
        Ok(scheduled(job_id))
    }
}

#[derive(Clone)]
struct NoopSubtitleSource;

#[async_trait]
impl SubtitleSourcePort for NoopSubtitleSource {
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
async fn spawned_runner_panic_is_durably_terminalized() {
    let scheduler = Arc::new(PanicScheduler {
        fail_records: Arc::new(Mutex::new(Vec::new())),
    });
    let runner = MockDubbingPipelineRunner::new(
        scheduler.clone(),
        InMemoryProjectRepository::new(Arc::new(std::sync::Mutex::new(
            adapters_storage::memory::InMemoryDatabase::new(),
        ))),
        NoopSubtitleSource,
        MockStorageUnitOfWork::new(),
        MockArtifactStore,
        Arc::new(LocalTempWorkspace::new(std::env::temp_dir())),
        Arc::new(NoopRuntime),
    )
    .with_panic_on_run();
    let (cancel_handle, token) = ports::cancellation::CancelHandle::new();
    let completion = Arc::new(ports::job_runtime_control::RuntimeCompletion::new());
    let (activate_tx, activate_rx) = tokio::sync::oneshot::channel();
    let (ack_tx, ack_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let task = super::super::start_mock_runtime::spawn_mock_pipeline_task(
        runner,
        scheduler.clone(),
        Arc::new(NoopRuntime),
        JobId::new(),
        ProjectId::new(),
        cancel_handle,
        token,
        completion,
        activate_rx,
        ack_tx,
        release_rx,
        tracing::info_span!("panic_test"),
    );

    activate_tx.send(()).unwrap();
    ack_rx.await.unwrap();
    release_tx.send(()).unwrap();

    let outcome = task.join_handle.await.unwrap();
    assert_eq!(outcome, RuntimeTaskOutcome::ApplicationFailed);
    assert_eq!(
        scheduler.fail_records.lock().await.as_slice(),
        &["RUNNER_PANIC"]
    );
}

fn scheduled(job_id: &JobId) -> ScheduledJob {
    ScheduledJob {
        id: job_id.clone(),
        revision: 1,
        project_id: None,
        title: "Job".into(),
        status: domain::job::JobStatus::Running,
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
