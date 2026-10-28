use domain::job::JobId;
use domain::project::ProjectId;
use ports::job_runtime_control::RuntimeTask;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub enum JobRuntimeEntry {
    Managed {
        project_id: ProjectId,
        task: RuntimeTask,
        /// Keeps resources owned even if the asynchronous supervisor unexpectedly unwinds.
        resource_guard: Arc<dyn Send + Sync>,
    },
    Reserved {
        project_id: ProjectId,
    },
    Attached {
        project_id: ProjectId,
        task: RuntimeTask,
    },
}

pub struct RuntimeRegistryInner {
    pub(crate) entries: HashMap<JobId, JobRuntimeEntry>,
    pub(crate) closed: bool,
}

#[derive(Clone)]
pub struct RuntimeRegistry {
    pub(crate) inner: Arc<Mutex<RuntimeRegistryInner>>,
}

impl RuntimeRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RuntimeRegistryInner {
                entries: HashMap::new(),
                closed: false,
            })),
        }
    }

    pub fn lock_inner(&self) -> std::sync::MutexGuard<'_, RuntimeRegistryInner> {
        match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::error!("runtime registry mutex was poisoned");
                poisoned.into_inner()
            }
        }
    }

    pub fn lock_entries(&self) -> std::sync::MutexGuard<'_, RuntimeRegistryInner> {
        self.lock_inner()
    }

    pub fn close_and_take_all(&self) -> HashMap<JobId, JobRuntimeEntry> {
        self.try_close().unwrap_or_default() // allow-fallback: an already-closed registry has no live entries
    }

    pub fn try_close(&self) -> Option<HashMap<JobId, JobRuntimeEntry>> {
        let mut inner = self.lock_inner();
        if inner.closed {
            None
        } else {
            inner.closed = true;
            let mut legacy = std::mem::take(&mut inner.entries);
            let managed: Vec<_> = legacy
                .iter()
                .filter_map(|(id, entry)| {
                    matches!(entry, JobRuntimeEntry::Managed { .. }).then_some(id.clone())
                })
                .collect();
            for id in managed {
                if let Some(entry) = legacy.remove(&id) {
                    inner.entries.insert(id, entry);
                }
            }
            Some(legacy)
        }
    }
}

impl Default for RuntimeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
