use domain::job::JobId as DomainJobId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Default)]
pub struct JobMutationLocks {
    locks: Arc<std::sync::Mutex<HashMap<DomainJobId, Arc<Mutex<()>>>>>,
}

impl JobMutationLocks {
    pub fn new() -> Self {
        Self {
            locks: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }

    pub fn get_lock(&self, job_id: &DomainJobId) -> Arc<Mutex<()>> {
        let mut map = self.locks.lock().unwrap();
        if let Some(lock) = map.get(job_id) {
            lock.clone()
        } else {
            let lock = Arc::new(Mutex::new(()));
            map.insert(job_id.clone(), lock.clone());
            lock
        }
    }

    pub fn remove(&self, job_id: &DomainJobId) {
        let mut map = self.locks.lock().unwrap();
        map.remove(job_id);
    }
}
