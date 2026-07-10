use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use domain::job::JobId;

#[derive(Clone)]
pub struct CancellationRegistry {
    handles: Arc<RwLock<HashMap<JobId, crate::cancellation::CancelHandle>>>,
}

impl CancellationRegistry {
    pub fn new() -> Self {
        Self {
            handles: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, id: JobId, handle: crate::cancellation::CancelHandle) {
        self.handles.write().await.insert(id, handle);
    }

    pub async fn unregister(&self, id: &JobId) {
        self.handles.write().await.remove(id);
    }

    pub async fn cancel(&self, id: &JobId) {
        let handles = self.handles.read().await;
        if let Some(handle) = handles.get(id) {
            handle.cancel();
        }
    }
}

impl Default for CancellationRegistry {
    fn default() -> Self {
        Self::new()
    }
}
