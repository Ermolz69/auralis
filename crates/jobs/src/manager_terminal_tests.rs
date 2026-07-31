use async_trait::async_trait;
use domain::job::{Job, JobId, JobKind, JobStatus};
use ports::error::PortError;
use ports::job_scheduler::{JobLifecycleEvent, JobSchedulerPort};
use ports::repository::JobRepository;
use ports::transaction::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::Mutex as AsyncMutex;

use crate::manager::JobManager;

struct Repo {
    jobs: Arc<AsyncMutex<HashMap<JobId, Job>>>,
}

#[async_trait]
impl JobRepository for Repo {
    async fn create(&self, job: Job) -> Result<Job, PortError> {
        self.jobs.lock().await.insert(job.id().clone(), job.clone());
        Ok(job)
    }

    async fn get(&self, id: &JobId) -> Result<Option<Job>, PortError> {
        Ok(self.jobs.lock().await.get(id).cloned())
    }

    async fn save(&self, job: &Job, _expected_revision: u64) -> Result<(), PortError> {
        self.jobs.lock().await.insert(job.id().clone(), job.clone());
        Ok(())
    }

    async fn list_by_project(
        &self,
        _project_id: &domain::project::ProjectId,
    ) -> Result<Vec<Job>, PortError> {
        Ok(vec![])
    }

    async fn list_active(&self) -> Result<Vec<Job>, PortError> {
        Ok(vec![])
    }

    async fn list_recent(&self, _limit: usize) -> Result<Vec<Job>, PortError> {
        Ok(vec![])
    }
}

struct FailingTerminalUow;

#[async_trait]
impl StorageUnitOfWork for FailingTerminalUow {
    async fn commit_transcript_import(&self, _c: CommitTranscriptImport) -> Result<(), PortError> {
        Ok(())
    }

    async fn commit_staged_artifact_write(
        &self,
        _c: CommitStagedArtifactWrite,
    ) -> Result<(), PortError> {
        Ok(())
    }

    async fn commit_artifact_finalize(
        &self,
        _c: CommitArtifactFinalize,
    ) -> Result<CommitArtifactFinalizeResult, PortError> {
        Ok(CommitArtifactFinalizeResult::Committed)
    }

    async fn commit_managed_source_import(
        &self,
        _c: CommitManagedSourceImport,
    ) -> Result<(), PortError> {
        Ok(())
    }

    async fn commit_project_delete(
        &self,
        _c: CommitProjectDelete,
    ) -> Result<CommitProjectDeleteResult, PortError> {
        Ok(CommitProjectDeleteResult {
            deleted_job_ids: vec![],
        })
    }

    async fn commit_job_update(&self, _c: CommitJobUpdate) -> Result<(), PortError> {
        Ok(())
    }

    async fn commit_pipeline_start(&self, _c: CommitPipelineStart) -> Result<(), PortError> {
        Ok(())
    }

    async fn commit_pipeline_start_failure(
        &self,
        _c: CommitPipelineStartFailure,
    ) -> Result<(), PortError> {
        Ok(())
    }

    async fn commit_terminal_job_update(
        &self,
        _c: CommitTerminalJobUpdate,
    ) -> Result<(), PortError> {
        Err(PortError::Storage {
            operation: "commit_terminal_job_update",
            message: "db down".into(),
        })
    }

    async fn apply_terminal_lifecycle_conditionally(
        &self,
        _c: ApplyTerminalLifecycle,
    ) -> Result<domain::project::status::TerminalTransitionResult, PortError> {
        Ok(domain::project::status::TerminalTransitionResult::AlreadyApplied)
    }
}

#[tokio::test]
async fn terminal_uow_failure_does_not_update_cache_or_emit_event() {
    let repo = Arc::new(Repo {
        jobs: Arc::new(AsyncMutex::new(HashMap::new())),
    });
    let mut job = Job::new(
        domain::project::ProjectId::new(),
        "Running".into(),
        JobKind::Dubbing,
    );
    job.start().unwrap();
    let job_id = job.id().clone();
    repo.create(job).await.unwrap();

    let events = Arc::new(Mutex::new(Vec::<JobLifecycleEvent>::new()));
    let event_sink = events.clone();
    let manager = JobManager::new(
        repo.clone(),
        Arc::new(FailingTerminalUow),
        Some(Arc::new(move |event| {
            event_sink.lock().unwrap().push(event)
        })),
    );

    let result = manager
        .fail_job(&job_id, "ERR".into(), "sanitized".into(), false)
        .await;

    assert!(matches!(result, Err(PortError::Storage { .. })));
    assert!(events.lock().unwrap().is_empty());
    let persisted = repo.get(&job_id).await.unwrap().unwrap();
    assert_eq!(persisted.status(), &JobStatus::Running);
}
