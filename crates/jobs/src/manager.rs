use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use chrono::Utc;

use crate::cancellation::CancelHandle;
use crate::error::JobError;
use crate::event::JobEvent;
use crate::id::JobId;
use crate::job::Job;
use crate::status::JobStatus;

pub type JobEventEmitter = Arc<dyn Fn(JobEvent) + Send + Sync + 'static>;

#[derive(Clone)]
pub struct JobManager {
    jobs: Arc<RwLock<HashMap<JobId, Job>>>,
    cancel_handles: Arc<RwLock<HashMap<JobId, CancelHandle>>>,
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

    pub async fn start_mock_dubbing_job(&self, title: String, project_id: Option<String>) -> JobId {
        let job = Job::new(title, project_id);
        let job_id = job.id.clone();

        {
            let mut jobs = self.jobs.write().await;
            jobs.insert(job_id.clone(), job.clone());
        }

        self.emit_job_event(&job);

        crate::mock_pipeline::run_mock_pipeline(self.clone(), job_id.clone());

        job_id
    }

    pub async fn list_jobs(&self) -> Vec<Job> {
        let jobs = self.jobs.read().await;
        let mut list: Vec<Job> = jobs.values().cloned().collect();
        // Sort by creation time descending (newest first)
        list.sort_by_key(|b| std::cmp::Reverse(b.created_at));
        list
    }

    pub async fn cancel_job(&self, id: &JobId) -> Result<Job, JobError> {
        let (job, should_cancel) = {
            let mut jobs = self.jobs.write().await;
            let job = jobs
                .get_mut(id)
                .ok_or_else(|| JobError::NotFound(id.clone()))?;

            let should_cancel = matches!(job.status, JobStatus::Queued | JobStatus::Running);
            if should_cancel {
                job.status = JobStatus::Cancelled;
                job.updated_at = Utc::now();
            }

            (job.clone(), should_cancel)
        };

        if should_cancel {
            let handles = self.cancel_handles.read().await;
            if let Some(handle) = handles.get(id) {
                handle.cancel();
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

    pub async fn get_job(&self, id: &JobId) -> Option<Job> {
        let jobs = self.jobs.read().await;
        jobs.get(id).cloned()
    }

    pub async fn register_cancel_handle(&self, id: JobId, handle: CancelHandle) {
        let mut handles = self.cancel_handles.write().await;
        handles.insert(id, handle);
    }

    pub async fn remove_cancel_handle(&self, id: &JobId) {
        let mut handles = self.cancel_handles.write().await;
        handles.remove(id);
    }

    fn emit_job_event(&self, job: &Job) {
        self.emit_event(JobEvent {
            job_id: job.id.clone(),
            project_id: job.project_id.clone(),
            status: job.status,
            stage: job.stage.clone(),
            progress: job.progress.clone(),
            message: None,
            error: job.error.clone(),
        });
    }

    fn emit_event(&self, event: JobEvent) {
        if let Some(emitter) = &self.emitter {
            emitter(event);
        }
    }
}
