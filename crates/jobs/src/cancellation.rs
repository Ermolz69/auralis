use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone)]
pub struct CancellationToken {
    is_cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new(is_cancelled: Arc<AtomicBool>) -> Self {
        Self { is_cancelled }
    }

    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone)]
pub struct CancelHandle {
    is_cancelled: Arc<AtomicBool>,
}

impl CancelHandle {
    pub fn new() -> (Self, CancellationToken) {
        let is_cancelled = Arc::new(AtomicBool::new(false));
        (
            Self {
                is_cancelled: is_cancelled.clone(),
            },
            CancellationToken::new(is_cancelled),
        )
    }

    pub fn cancel(&self) {
        self.is_cancelled.store(true, Ordering::SeqCst);
    }
}
