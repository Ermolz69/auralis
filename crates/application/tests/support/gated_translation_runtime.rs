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
    cancellable: bool,
    cleanup_fails: bool,
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
            cancellable: false,
            cleanup_fails: false,
        })
    }

    pub fn cancellable(endpoint: String) -> Arc<Self> {
        Arc::new(Self {
            entered: Semaphore::new(0),
            release: Semaphore::new(0),
            returned: AtomicUsize::new(0),
            dropped: Arc::new(AtomicUsize::new(0)),
            endpoint,
            fail: false,
            cancellable: true,
            cleanup_fails: false,
        })
    }

    pub fn cleanup_failure(endpoint: String) -> Arc<Self> {
        let mut runtime = Self::cancellable(endpoint);
        if let Some(runtime) = Arc::get_mut(&mut runtime) {
            runtime.cleanup_fails = true;
        }
        runtime
    }
}

#[async_trait::async_trait]
impl TranslationModelRuntimePort for GatedTranslationRuntime {
    async fn acquire_with_control(
        &self,
        _: &TranslationRunIntent,
        cancellation: ports::cancellation::CancellationToken,
    ) -> Result<Box<dyn TranslationModelLease>, PortError> {
        self.entered.add_permits(1);
        if self.cancellable {
            tokio::select! {
                _ = cancellation.cancelled() => return Err(if self.cleanup_fails {
                    PortError::CleanupFailed { resource: "test preparation".into(), message: "injected cleanup failure".into() }
                } else { PortError::Cancelled }),
                permit = self.release.acquire() => permit.map_err(|_| PortError::Cancelled)?.forget(),
            }
        } else {
            self.release
                .acquire()
                .await
                .map_err(|_| PortError::Cancelled)?
                .forget();
        }
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
