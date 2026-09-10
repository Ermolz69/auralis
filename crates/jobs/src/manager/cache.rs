use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

use domain::job::{Job, JobId, JobStatus};

#[derive(Clone)]
pub struct JobCache {
    active_job_ids: Arc<RwLock<HashSet<JobId>>>,
}

impl JobCache {
    pub fn new() -> Self {
        Self {
            active_job_ids: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub async fn track_persisted(&self, job: &Job) {
        let mut active_job_ids = self.active_job_ids.write().await;
        if matches!(
            job.status(),
            JobStatus::Pending | JobStatus::Running | JobStatus::Cancelling
        ) {
            active_job_ids.insert(job.id().clone());
        } else {
            active_job_ids.remove(job.id());
        }
    }

    pub async fn remove(&self, id: &JobId) {
        self.active_job_ids.write().await.remove(id);
    }

    pub async fn list_active_ids(&self) -> Vec<JobId> {
        self.active_job_ids.read().await.iter().cloned().collect()
    }

    #[cfg(test)]
    pub async fn active_count(&self) -> usize {
        self.active_job_ids.read().await.len()
    }

    #[cfg(test)]
    pub async fn contains(&self, job_id: &JobId) -> bool {
        self.active_job_ids.read().await.contains(job_id)
    }
}

impl Default for JobCache {
    fn default() -> Self {
        Self::new()
    }
}
