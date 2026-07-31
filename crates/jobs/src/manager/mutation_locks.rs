use domain::job::JobId as DomainJobId;
use ports::error::PortError;
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

    pub fn get_lock(&self, job_id: &DomainJobId) -> Result<Arc<Mutex<()>>, PortError> {
        let mut map = self.locks.lock().map_err(|_| PortError::Unexpected {
            message: "Job mutation lock map is unavailable".to_string(),
        })?;
        if let Some(lock) = map.get(job_id) {
            Ok(lock.clone())
        } else {
            let lock = Arc::new(Mutex::new(()));
            map.insert(job_id.clone(), lock.clone());
            Ok(lock)
        }
    }

    pub fn remove_if_unused(&self, job_id: &DomainJobId) -> Result<(), PortError> {
        let mut map = self.locks.lock().map_err(|_| PortError::Unexpected {
            message: "Job mutation lock map is unavailable".to_string(),
        })?;
        if map
            .get(job_id)
            .is_some_and(|lock| Arc::strong_count(lock) == 1)
        {
            map.remove(job_id);
        }
        Ok(())
    }

    pub fn release_if_unused(
        &self,
        job_id: &DomainJobId,
        lock: &Arc<Mutex<()>>,
    ) -> Result<(), PortError> {
        let mut map = self.locks.lock().map_err(|_| PortError::Unexpected {
            message: "Job mutation lock map is unavailable".to_string(),
        })?;
        if map
            .get(job_id)
            .is_some_and(|existing| Arc::ptr_eq(existing, lock) && Arc::strong_count(existing) <= 2)
        {
            map.remove(job_id);
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn poisoned_lock_map_returns_typed_failure() {
        let locks = JobMutationLocks::new();
        let clone = locks.clone();
        let _ = std::panic::catch_unwind(move || {
            let _guard = clone.locks.lock().unwrap();
            panic!("poison mutation lock map");
        });

        assert!(matches!(
            locks.get_lock(&DomainJobId::new()),
            Err(PortError::Unexpected { .. })
        ));
    }

    #[test]
    fn remove_if_unused_keeps_lock_with_concurrent_owner() {
        let locks = JobMutationLocks::new();
        let job_id = DomainJobId::new();
        let lock = locks.get_lock(&job_id).unwrap();

        locks.remove_if_unused(&job_id).unwrap();

        let current = locks.get_lock(&job_id).unwrap();
        assert!(Arc::ptr_eq(&lock, &current));
    }

    #[test]
    fn release_if_unused_removes_only_matching_unshared_lock() {
        let locks = JobMutationLocks::new();
        let job_id = DomainJobId::new();
        let lock = locks.get_lock(&job_id).unwrap();

        locks.release_if_unused(&job_id, &lock).unwrap();
        let replacement = locks.get_lock(&job_id).unwrap();

        assert!(!Arc::ptr_eq(&lock, &replacement));
    }
}
