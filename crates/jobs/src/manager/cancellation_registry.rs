use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use domain::job::JobId;

use tokio::sync::oneshot;

pub struct JobRegistration {
    pub cancel_handle: crate::cancellation::CancelHandle,
    pub completion_rx: Option<oneshot::Receiver<()>>,
}

#[derive(Clone)]
pub struct CancellationRegistry {
    handles: Arc<RwLock<HashMap<JobId, JobRegistration>>>,
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
        handle: crate::cancellation::CancelHandle,
        completion_rx: Option<oneshot::Receiver<()>>,
    ) {
        self.handles.write().await.insert(
            id,
            JobRegistration {
                cancel_handle: handle,
                completion_rx,
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

    pub async fn cancel_and_wait(&self, id: &JobId) {
        let mut rx_to_wait = None;
        {
            let mut handles = self.handles.write().await;
            if let Some(reg) = handles.get_mut(id) {
                reg.cancel_handle.cancel();
                rx_to_wait = reg.completion_rx.take();
            }
        }
        if let Some(rx) = rx_to_wait {
            let _ = rx.await;
        }
    }
}

impl Default for CancellationRegistry {
    fn default() -> Self {
        Self::new()
    }
}
