use ports::{
    error::PortError,
    translation_links::TranslationRunIntent,
    translation_model_runtime::{TranslationModelLease, TranslationModelRuntimePort},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::sync::Semaphore;

#[path = "observed_translation_lease.rs"]
mod observed_translation_lease;

pub struct GatedTranslationRuntime {
    pub entered: Semaphore,
    pub release: Semaphore,
    pub returned: AtomicUsize,
    pub dropped: Arc<AtomicUsize>,
    endpoint: String,
    fail: bool,
}

impl GatedTranslationRuntime {
    pub fn new(endpoint: String) -> Arc<Self> {
        Self::configured(endpoint, false)
    }

    pub fn failing(endpoint: String) -> Arc<Self> {
        Self::configured(endpoint, true)
    }

    fn configured(endpoint: String, fail: bool) -> Arc<Self> {
        Arc::new(Self {
            entered: Semaphore::new(0),
            release: Semaphore::new(0),
            returned: AtomicUsize::new(0),
            dropped: Arc::new(AtomicUsize::new(0)),
            endpoint,
            fail,
        })
    }
}

#[async_trait::async_trait]
impl TranslationModelRuntimePort for GatedTranslationRuntime {
    async fn acquire(
        &self,
        _: &TranslationRunIntent,
    ) -> Result<Box<dyn TranslationModelLease>, PortError> {
        self.entered.add_permits(1);
        self.release
            .acquire()
            .await
            .map_err(|_| PortError::Cancelled)?
            .forget();
        self.returned.fetch_add(1, Ordering::SeqCst);
        if self.fail {
            return Err(PortError::ExternalToolFailed {
                tool: "gated test runtime".into(),
                message: "runtime admission failed".into(),
            });
        }
        Ok(Box::new(
            observed_translation_lease::ObservedTranslationLease {
                endpoint: self.endpoint.clone(),
                dropped: self.dropped.clone(),
            },
        ))
    }
}
