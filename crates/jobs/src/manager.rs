use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;

use chrono::Utc;

use crate::error::JobError;
use crate::id::JobId;
use crate::job::Job;
use domain::job::JobStatus;

use async_trait::async_trait;
use domain::dubbing::DubbingPipelineStage;
use domain::job::JobId as DomainJobId;
use domain::project::ProjectId as DomainProjectId;
use ports::job_scheduler::JobLifecycleEvent;

use ports::error::PortError;
use ports::job_scheduler::{JobSchedulerPort, ScheduledJob, StartDubbingJobRequest};

pub type JobEventEmitter = Arc<dyn Fn(JobLifecycleEvent) + Send + Sync + 'static>;

#[derive(Clone)]
pub struct JobManager {
    jobs: Arc<RwLock<HashMap<JobId, Job>>>,
    cancel_handles: Arc<RwLock<HashMap<JobId, crate::cancellation::CancelHandle>>>,
    emitter: Option<JobEventEmitter>,
}

impl JobManager {
    pub fn new(emitter: Option<JobEventEmitter>) -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            cancel_handles: Arc::new(RwLock::new(HashMap::new())),
            emitter,
        }
    }

    pub async fn start_mock_dubbing_job_internal(
        &self,
        title: String,
        project_id: Option<String>,
    ) -> JobId {
        let job = Job::new(title, project_id);
        let job_id = job.id.clone();

        self.jobs.write().await.insert(job_id.clone(), job.clone());

        self.emit_job_event(&job);

        crate::mock_pipeline::run_mock_pipeline(self.clone(), job_id.clone());

        job_id
    }

    pub async fn get_job_internal(&self, job_id: &JobId) -> Option<Job> {
        self.jobs.read().await.get(job_id).cloned()
    }

    pub async fn list_jobs_internal(&self) -> Vec<Job> {
        let mut jobs: Vec<Job> = self.jobs.read().await.values().cloned().collect();
        // Sort by creation time, newest first
        jobs.sort_by_key(|b| std::cmp::Reverse(b.created_at));
        jobs
    }

    pub async fn cancel_job_internal(&self, job_id: &JobId) -> Result<Job, JobError> {
        let (job, should_cancel) = {
            let mut jobs = self.jobs.write().await;
            let job = jobs
                .get_mut(job_id)
                .ok_or_else(|| JobError::NotFound(job_id.clone()))?;

            let should_cancel = matches!(job.status, JobStatus::Pending | JobStatus::Running);
            if should_cancel {
                job.status = JobStatus::Cancelled;
                job.updated_at = Utc::now();
            }

            (job.clone(), should_cancel)
        };

        if should_cancel {
            {
                let handles = self.cancel_handles.read().await;
                if let Some(handle) = handles.get(job_id) {
                    handle.cancel();
                }
            }

            self.emit_job_event(&job);
        }

        Ok(job)
    }

    pub async fn update_job(&self, updated_job: Job) {
        {
            let mut jobs = self.jobs.write().await;
            jobs.insert(updated_job.id.clone(), updated_job.clone());
        }

        self.emit_job_event(&updated_job);
    }

    pub async fn register_cancel_handle(
        &self,
        id: JobId,
        handle: crate::cancellation::CancelHandle,
    ) {
        let mut handles = self.cancel_handles.write().await;
        handles.insert(id, handle);
    }

    pub async fn remove_cancel_handle(&self, id: &JobId) {
        let mut handles = self.cancel_handles.write().await;
        handles.remove(id);
    }

    pub async fn update_job_status(&self, job_id: &JobId, status: JobStatus) {
        let job_to_emit = {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(job_id) {
                job.status = status;
                job.updated_at = Utc::now();
                Some(job.clone())
            } else {
                None
            }
        };

        if let Some(job) = job_to_emit {
            self.emit_job_event(&job);
        }
    }

    pub async fn update_job_progress(&self, job_id: &JobId, percent: u8) {
        let job_to_emit = {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(job_id) {
                job.progress.percent = percent;
                job.updated_at = Utc::now();
                Some(job.clone())
            } else {
                None
            }
        };

        if let Some(job) = job_to_emit {
            self.emit_job_event(&job);
        }
    }

    pub async fn update_job_stage(&self, job_id: &JobId, stage: crate::stage::JobStage) {
        let job_to_emit = {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(job_id) {
                job.stage = Some(stage);
                job.updated_at = Utc::now();
                Some(job.clone())
            } else {
                None
            }
        };

        if let Some(job) = job_to_emit {
            self.emit_job_event(&job);
        }
    }

    pub async fn fail_job(&self, job_id: &JobId, error: String) {
        let job_to_emit = {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(job_id) {
                job.status = JobStatus::Failed;
                job.error = Some(error);
                job.updated_at = Utc::now();
                Some(job.clone())
            } else {
                None
            }
        };

        if let Some(job) = job_to_emit {
            self.emit_job_event(&job);
        }
    }

    fn map_job_to_scheduled(job: &Job) -> ScheduledJob {
        let project_id = job
            .project_id
            .as_ref()
            .and_then(|id| DomainProjectId::from_str(id).ok());
        let domain_id = DomainJobId::from_str(&job.id.0).unwrap_or_default();

        let status = job.status.clone();

        let stage = match &job.stage {
            Some(crate::stage::JobStage::ValidateSource) => {
                Some(DubbingPipelineStage::ValidateSource)
            }
            Some(crate::stage::JobStage::FetchMetadata) => {
                Some(DubbingPipelineStage::FetchMetadata)
            }
            Some(crate::stage::JobStage::PrepareMedia) => Some(DubbingPipelineStage::DownloadMedia),
            Some(crate::stage::JobStage::GenerateTranscript) => {
                Some(DubbingPipelineStage::ExtractOrGenerateTranscript)
            }
            Some(crate::stage::JobStage::Finalize) => Some(DubbingPipelineStage::ExportResult),
            None => None,
        };

        let progress = domain::job::JobProgress {
            percent: job.progress.percent,
            message: "Mock processing...".to_string(),
            current_step: None,
            processed_items: None,
            total_items: None,
        };

        ScheduledJob {
            id: domain_id,
            project_id,
            title: job.title.clone(),
            status,
            stage,
            progress,
            error: job.error.clone(),
            created_at: job.created_at,
            updated_at: job.updated_at,
        }
    }

    fn emit_job_event(&self, job: &Job) {
        let scheduled = Self::map_job_to_scheduled(job);

        let event = JobLifecycleEvent {
            job_id: scheduled.id,
            project_id: scheduled.project_id,
            status: scheduled.status,
            stage: scheduled.stage,
            progress: scheduled.progress,
            error: scheduled.error,
        };

        if let Some(emitter) = &self.emitter {
            emitter(event);
        }
    }
}

#[async_trait]
impl JobSchedulerPort for JobManager {
    async fn start_dubbing_job(
        &self,
        request: StartDubbingJobRequest,
    ) -> Result<ScheduledJob, PortError> {
        let project_id_str = request.project_id.map(|id| id.to_string());
        let job_id = self
            .start_mock_dubbing_job_internal(request.title, project_id_str)
            .await;

        let job = self.get_job_internal(&job_id).await.unwrap();
        Ok(Self::map_job_to_scheduled(&job))
    }

    async fn cancel_job(&self, job_id: &DomainJobId) -> Result<ScheduledJob, PortError> {
        let adapter_id = JobId(job_id.to_string());
        let job =
            self.cancel_job_internal(&adapter_id)
                .await
                .map_err(|e| PortError::Unexpected {
                    message: e.to_string(),
                })?;
        Ok(Self::map_job_to_scheduled(&job))
    }

    async fn get_job(&self, job_id: &DomainJobId) -> Result<Option<ScheduledJob>, PortError> {
        let adapter_id = JobId(job_id.to_string());
        let job = self.get_job_internal(&adapter_id).await;
        Ok(job.map(|j| Self::map_job_to_scheduled(&j)))
    }

    async fn list_jobs(&self) -> Result<Vec<ScheduledJob>, PortError> {
        let jobs = self.list_jobs_internal().await;
        Ok(jobs
            .into_iter()
            .map(|j| Self::map_job_to_scheduled(&j))
            .collect())
    }
}
