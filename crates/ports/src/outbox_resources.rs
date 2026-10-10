use std::sync::Arc;

pub trait OutboxResourceGate: Send + Sync {
    fn try_acquire_disk(&self) -> Option<Arc<dyn Send + Sync>>;
}
