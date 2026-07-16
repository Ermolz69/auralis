use domain::job::JobId;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, watch};
use tokio::task::AbortHandle;

pub struct JobRegistration {
    pub cancel_handle: ports::cancellation::CancelHandle,
    pub state_rx: watch::Receiver<ports::job_runtime_control::RuntimeState>,
    pub abort_handle: AbortHandle,
}

#[derive(Clone)]
pub struct CancellationRegistry {
    pub(crate) handles: Arc<RwLock<HashMap<JobId, JobRegistration>>>,
}

impl CancellationRegistry {
    pub fn new() -> Self {
        Self {
            handles: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(
        &self,
        id: JobId,
        cancel_handle: ports::cancellation::CancelHandle,
        state_rx: watch::Receiver<ports::job_runtime_control::RuntimeState>,
        abort_handle: AbortHandle,
    ) {
        self.handles.write().await.insert(
            id,
            JobRegistration {
                cancel_handle,
                state_rx,
                abort_handle,
            },
        );
    }

    pub async fn unregister(&self, id: &JobId) {
        self.handles.write().await.remove(id);
    }

    pub async fn cancel(&self, id: &JobId) {
        let handles = self.handles.read().await;
        if let Some(reg) = handles.get(id) {
            reg.cancel_handle.cancel();
        }
    }
}

impl Default for CancellationRegistry {
    fn default() -> Self {
        Self::new()
    }
}
