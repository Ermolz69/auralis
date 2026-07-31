use domain::project::ProjectId;
use ports::error::PortError;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};

pub struct ProjectLifecycleLocks {
    locks: Mutex<HashMap<ProjectId, Weak<tokio::sync::Mutex<()>>>>,
}

impl ProjectLifecycleLocks {
    pub fn new() -> Self {
        Self {
            locks: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for ProjectLifecycleLocks {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectLifecycleLocks {
    pub fn get_lock(
        &self,
        project_id: &ProjectId,
    ) -> Result<Arc<tokio::sync::Mutex<()>>, PortError> {
        let mut locks = self.locks.lock().map_err(|_| PortError::Storage {
            operation: "lock_project_lifecycle",
            message: "Mutex poisoned".to_string(),
        })?;

        if let Some(weak_lock) = locks.get(project_id)
            && let Some(arc_lock) = weak_lock.upgrade()
        {
            return Ok(arc_lock);
        }

        let new_lock = Arc::new(tokio::sync::Mutex::new(()));
        locks.insert(project_id.clone(), Arc::downgrade(&new_lock));

        // Periodically cleanup dead weak references when we access to keep memory small.
        // Or just clean up inline if we reach a certain size, but hashmap overhead is minimal.
        if locks.len() > 1000 {
            locks.retain(|_, weak| weak.strong_count() > 0);
        }

        Ok(new_lock)
    }
}
