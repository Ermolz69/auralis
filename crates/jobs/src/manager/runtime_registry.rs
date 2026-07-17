use domain::job::JobId;
use domain::project::ProjectId;
use ports::job_runtime_control::RuntimeTask;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub enum JobRuntimeEntry {
    Reserved {
        project_id: ProjectId,
    },
    Attached {
        project_id: ProjectId,
        task: RuntimeTask,
    },
}

#[derive(Clone)]
pub struct RuntimeRegistry {
    pub(crate) handles: Arc<Mutex<HashMap<JobId, JobRuntimeEntry>>>,
}

impl RuntimeRegistry {
    pub fn new() -> Self {
        Self {
            handles: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn lock_entries(&self) -> std::sync::MutexGuard<'_, HashMap<JobId, JobRuntimeEntry>> {
        match self.handles.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::error!("runtime registry mutex was poisoned");
                poisoned.into_inner()
            }
        }
    }
}

impl Default for RuntimeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
