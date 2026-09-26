use ports::translation_model_runtime::TranslationModelLease;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

pub struct ObservedTranslationLease {
    pub endpoint: String,
    pub dropped: Arc<AtomicUsize>,
}

impl TranslationModelLease for ObservedTranslationLease {
    fn server_url(&self) -> &str {
        &self.endpoint
    }
}

impl Drop for ObservedTranslationLease {
    fn drop(&mut self) {
        self.dropped.fetch_add(1, Ordering::SeqCst);
    }
}
