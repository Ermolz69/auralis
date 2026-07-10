use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use domain::job::{Job, JobId};

#[derive(Clone)]
pub struct JobCache {
    jobs: Arc<RwLock<HashMap<JobId, Job>>>,
}

impl JobCache {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn insert(&self, job: Job) {
        self.jobs.write().await.insert(job.id().clone(), job);
    }

    pub async fn get(&self, id: &JobId) -> Option<Job> {
        self.jobs.read().await.get(id).cloned()
    }

    pub async fn list_all(&self) -> Vec<Job> {
        self.jobs.read().await.values().cloned().collect()
    }
}

impl Default for JobCache {
    fn default() -> Self {
        Self::new()
    }
}
