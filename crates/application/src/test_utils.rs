use async_trait::async_trait;
use chrono::Utc;
use domain::job::JobId;
use ports::error::PortError;
use ports::job_scheduler::{JobSchedulerPort, ScheduledJob, StartDubbingJobRequest};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct MockJobScheduler {
    pub jobs: Arc<Mutex<Vec<ScheduledJob>>>,
}

impl Default for MockJobScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl MockJobScheduler {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl JobSchedulerPort for MockJobScheduler {
    async fn start_dubbing_job(
        &self,
        request: StartDubbingJobRequest,
    ) -> Result<ScheduledJob, PortError> {
        let job = ScheduledJob {
            id: JobId::new(),
            project_id: request.project_id,
            title: request.title,
            status: domain::job::JobStatus::Pending,
            stage: None,
            progress: domain::job::JobProgress::initializing(),
            error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mut jobs = self.jobs.lock().await;
        jobs.push(job.clone());

        Ok(job)
    }

    async fn enqueue_existing_job(&self, job_id: &JobId) -> Result<ScheduledJob, PortError> {
        let job = ScheduledJob {
            id: job_id.clone(),
            project_id: None,
            title: "Mock Enqueued Job".to_string(),
            status: domain::job::JobStatus::Running,
            stage: None,
            progress: domain::job::JobProgress::initializing(),
            error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mut jobs = self.jobs.lock().await;
        jobs.push(job.clone());

        Ok(job)
    }

    async fn cancel_job(&self, job_id: &JobId) -> Result<ScheduledJob, PortError> {
        let mut jobs = self.jobs.lock().await;
        if let Some(job) = jobs.iter_mut().find(|j| j.id == *job_id) {
            job.status = domain::job::JobStatus::Cancelled;
            job.updated_at = Utc::now();
            return Ok(job.clone());
        }
        Err(PortError::NotFound {
            resource: format!("Job {}", job_id),
        })
    }

    async fn get_job(&self, job_id: &JobId) -> Result<Option<ScheduledJob>, PortError> {
        let jobs = self.jobs.lock().await;
        Ok(jobs.iter().find(|j| j.id == *job_id).cloned())
    }

    async fn list_jobs(&self) -> Result<Vec<ScheduledJob>, PortError> {
        let jobs = self.jobs.lock().await;
        Ok(jobs.clone())
    }
}

use domain::media::{Artifact, ArtifactKind};
use domain::project::ProjectId;
use ports::storage::{ArtifactStore, StagedArtifact};
use std::path::Path;

#[derive(Clone, Default)]
pub struct MockArtifactStore;

#[async_trait]
impl ArtifactStore for MockArtifactStore {
    async fn write_small_artifact(
        &self,
        _project_id: &ProjectId,
        _kind: ArtifactKind,
        _filename: &str,
        _data: &[u8],
    ) -> Result<Artifact, PortError> {
        unimplemented!()
    }

    async fn stage_external_file(
        &self,
        _project_id: &ProjectId,
        _kind: ArtifactKind,
        _source_path: &Path,
        _filename_hint: Option<&str>,
    ) -> Result<StagedArtifact, PortError> {
        unimplemented!()
    }

    async fn finalize_staged_artifact(
        &self,
        _staging_key: &str,
        _final_key: &str,
    ) -> Result<(), PortError> {
        unimplemented!()
    }

    async fn resolve_artifact(
        &self,
        _artifact: &Artifact,
    ) -> Result<std::path::PathBuf, PortError> {
        unimplemented!()
    }

    async fn delete_storage_key(&self, _key: &str) -> Result<(), PortError> {
        Ok(())
    }

    async fn delete_artifact(&self, _artifact: &Artifact) -> Result<(), PortError> {
        Ok(())
    }

    async fn delete_project_dir(&self, _project_id: &ProjectId) -> Result<(), PortError> {
        Ok(())
    }
}

use ports::transaction::{TransactionGateway, UnitOfWorkData};

#[derive(Clone)]
pub struct MockTransactionGateway {
    pub should_fail: bool,
    pub projects_saved: Arc<Mutex<Vec<domain::project::Project>>>,
    pub jobs_saved: Arc<Mutex<Vec<domain::job::Job>>>,
}

impl Default for MockTransactionGateway {
    fn default() -> Self {
        Self::new()
    }
}

impl MockTransactionGateway {
    pub fn new() -> Self {
        Self {
            should_fail: false,
            projects_saved: Arc::new(Mutex::new(Vec::new())),
            jobs_saved: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_failure() -> Self {
        Self {
            should_fail: true,
            projects_saved: Arc::new(Mutex::new(Vec::new())),
            jobs_saved: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl TransactionGateway for MockTransactionGateway {
    async fn execute(&self, data: UnitOfWorkData) -> Result<(), PortError> {
        if self.should_fail {
            return Err(PortError::Unexpected {
                message: "Mock transaction failure".to_string(),
            });
        }

        let mut projects = self.projects_saved.lock().await;
        projects.extend(data.projects_to_save);

        let mut jobs = self.jobs_saved.lock().await;
        jobs.extend(data.jobs_to_save);

        Ok(())
    }
}
