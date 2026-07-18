use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Notify;

#[derive(Debug, Clone)]
pub struct CancellationToken {
    is_cancelled: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl CancellationToken {
    pub fn new(is_cancelled: Arc<AtomicBool>, notify: Arc<Notify>) -> Self {
        Self {
            is_cancelled,
            notify,
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::SeqCst)
    }

    pub async fn cancelled(&self) {
        if self.is_cancelled() {
            return;
        }
        self.notify.notified().await;
    }
}

#[derive(Debug, Clone)]
pub struct CancelHandle {
    is_cancelled: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl CancelHandle {
    pub fn new() -> (Self, CancellationToken) {
        let is_cancelled = Arc::new(AtomicBool::new(false));
        let notify = Arc::new(Notify::new());
        (
            Self {
                is_cancelled: is_cancelled.clone(),
                notify: notify.clone(),
            },
            CancellationToken::new(is_cancelled, notify),
        )
    }

    pub fn cancel(&self) {
        self.is_cancelled.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::SeqCst)
    }
}
